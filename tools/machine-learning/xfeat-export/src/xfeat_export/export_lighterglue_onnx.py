from __future__ import annotations

import os
from pathlib import Path

import click
import torch
import torch.nn.functional as F
from torch import Tensor, nn

from modules.lighterglue import LighterGlue


def default_weights_path() -> Path:
    import modules.model

    return Path(modules.model.__file__).resolve().parents[1] / "weights" / "xfeat-lighterglue.pt"


def disable_dynamic_attention(module: nn.Module) -> None:
    for child in module.modules():
        if hasattr(child, "enable_flash"):
            child.enable_flash = False
        if hasattr(child, "has_sdp"):
            child.has_sdp = False
        if child.__class__.__name__ == "CrossBlock" and hasattr(child, "flash"):
            child.flash = None


def filter_matches(scores: Tensor, threshold: float) -> tuple[Tensor, Tensor, Tensor, Tensor]:
    max0 = scores.max(dim=2)
    max1 = scores.max(dim=1)

    matches0 = max0.indices
    matches1 = max1.indices
    indices0 = torch.arange(matches0.shape[1], device=matches0.device)[None]
    indices1 = torch.arange(matches1.shape[1], device=matches1.device)[None]

    mutual0 = indices0 == matches1.gather(1, matches0)
    mutual1 = indices1 == matches0.gather(1, matches1)

    match_scores0 = max0.values.exp()
    zero = match_scores0.new_tensor(0.0)
    match_scores0 = torch.where(mutual0, match_scores0, zero)
    match_scores1 = torch.where(mutual1, match_scores0.gather(1, matches1), zero)

    valid0 = mutual0 & (match_scores0 > threshold)
    valid1 = mutual1 & valid0.gather(1, matches1)
    minus_one = matches0.new_tensor(-1)

    matches0 = torch.where(valid0, matches0, minus_one)
    matches1 = torch.where(valid1, matches1, minus_one)
    return matches0, matches1, match_scores0, match_scores1


def rotate_half(x: Tensor) -> Tensor:
    shape = x.shape
    paired = x.reshape(shape[0], shape[1], shape[2], shape[3] // 2, 2)
    return torch.stack((-paired[..., 1], paired[..., 0]), dim=4).reshape(shape)


def apply_rotary_encoding(encoding: Tensor, x: Tensor) -> Tensor:
    return (x * encoding[0]) + (rotate_half(x) * encoding[1])


def scaled_attention(query: Tensor, key: Tensor, value: Tensor, mask: Tensor) -> Tensor:
    similarity = torch.einsum("bhid,bhjd->bhij", query, key) * query.shape[3] ** -0.5
    attention = torch.softmax(similarity.masked_fill(~mask, -1.0e9), dim=3)
    return torch.einsum("bhij,bhjd->bhid", attention, value)


def exportable_self_attention(block: nn.Module, x: Tensor, encoding: Tensor, valid: Tensor) -> Tensor:
    batch_size, keypoint_count, _ = x.shape
    qkv = block.Wqkv(x).reshape(batch_size, keypoint_count, block.num_heads, block.head_dim, 3)
    qkv = qkv.permute(0, 2, 1, 3, 4)

    query = apply_rotary_encoding(encoding, qkv[..., 0])
    key = apply_rotary_encoding(encoding, qkv[..., 1])
    value = qkv[..., 2]
    mask = valid[:, None, :, None] & valid[:, None, None, :]

    context = scaled_attention(query, key, value, mask)
    context = context.permute(0, 2, 1, 3).reshape(batch_size, keypoint_count, block.num_heads * block.head_dim)
    message = block.out_proj(context)
    return x + block.ffn(torch.cat([x, message], dim=2))


def exportable_cross_attention(
    block: nn.Module,
    x0: Tensor,
    x1: Tensor,
    valid0: Tensor,
    valid1: Tensor,
) -> tuple[Tensor, Tensor]:
    batch_size, keypoint_count0, _ = x0.shape
    _, keypoint_count1, _ = x1.shape
    head_count = block.heads
    head_dim = block.to_qk.out_features // head_count

    qk0 = block.to_qk(x0).reshape(batch_size, keypoint_count0, head_count, head_dim).permute(0, 2, 1, 3)
    qk1 = block.to_qk(x1).reshape(batch_size, keypoint_count1, head_count, head_dim).permute(0, 2, 1, 3)
    v0 = block.to_v(x0).reshape(batch_size, keypoint_count0, head_count, head_dim).permute(0, 2, 1, 3)
    v1 = block.to_v(x1).reshape(batch_size, keypoint_count1, head_count, head_dim).permute(0, 2, 1, 3)

    qk0 = qk0 * block.scale**0.5
    qk1 = qk1 * block.scale**0.5
    similarity = torch.einsum("bhid,bhjd->bhij", qk0, qk1)

    mask01 = valid0[:, None, :, None] & valid1[:, None, None, :]
    attention01 = torch.softmax(similarity.masked_fill(~mask01, -1.0e9), dim=3)
    message0 = torch.einsum("bhij,bhjd->bhid", attention01, v1)

    mask10 = mask01.permute(0, 1, 3, 2)
    attention10 = torch.softmax(similarity.permute(0, 1, 3, 2).masked_fill(~mask10, -1.0e9), dim=3)
    message1 = torch.einsum("bhij,bhjd->bhid", attention10, v0)

    message0 = message0.permute(0, 2, 1, 3).reshape(batch_size, keypoint_count0, head_count * head_dim)
    message1 = message1.permute(0, 2, 1, 3).reshape(batch_size, keypoint_count1, head_count * head_dim)
    message0 = block.to_out(message0)
    message1 = block.to_out(message1)

    x0 = x0 + block.ffn(torch.cat([x0, message0], dim=2))
    x1 = x1 + block.ffn(torch.cat([x1, message1], dim=2))
    return x0, x1


def exportable_transformer_layer(
    transformer: nn.Module,
    desc0: Tensor,
    desc1: Tensor,
    encoding0: Tensor,
    encoding1: Tensor,
    valid0: Tensor,
    valid1: Tensor,
) -> tuple[Tensor, Tensor]:
    desc0 = exportable_self_attention(transformer.self_attn, desc0, encoding0, valid0)
    desc1 = exportable_self_attention(transformer.self_attn, desc1, encoding1, valid1)
    return exportable_cross_attention(transformer.cross_attn, desc0, desc1, valid0, valid1)


def exportable_assignment_scores(
    assignment: nn.Module,
    desc0: Tensor,
    desc1: Tensor,
    valid0: Tensor,
    valid1: Tensor,
) -> Tensor:
    projected0 = assignment.final_proj(desc0)
    projected1 = assignment.final_proj(desc1)
    descriptor_dim = projected0.shape[2]
    projected0 = projected0 / descriptor_dim**0.25
    projected1 = projected1 / descriptor_dim**0.25

    similarity = torch.einsum("bmd,bnd->bmn", projected0, projected1)
    matchability0 = assignment.matchability(desc0)
    matchability1 = assignment.matchability(desc1)
    certainties = F.logsigmoid(matchability0) + F.logsigmoid(matchability1).permute(0, 2, 1)
    scores0 = F.log_softmax(similarity.masked_fill(~valid1[:, None, :], -1.0e9), dim=2)
    scores1 = F.log_softmax(
        similarity.permute(0, 2, 1).contiguous().masked_fill(~valid0[:, None, :], -1.0e9),
        dim=2,
    ).permute(0, 2, 1)
    scores = scores0 + scores1 + certainties
    return scores.masked_fill(~(valid0[:, :, None] & valid1[:, None, :]), -1.0e9)


class LighterGlueFixedWrapper(nn.Module):
    def __init__(self, weights_path: Path, *, min_confidence: float) -> None:
        super().__init__()
        self.min_confidence = min_confidence
        self.matcher = LighterGlue(weights=str(weights_path)).net
        self.matcher.conf.mp = False
        self.matcher.conf.depth_confidence = -1
        self.matcher.conf.width_confidence = -1
        self.matcher.conf.filter_threshold = min_confidence
        disable_dynamic_attention(self.matcher)

    def forward(
        self,
        keypoints0: Tensor,
        keypoints1: Tensor,
        descriptors0: Tensor,
        descriptors1: Tensor,
        valid0: Tensor,
        valid1: Tensor,
    ) -> tuple[Tensor, Tensor, Tensor, Tensor]:
        desc0 = self.matcher.input_proj(descriptors0.contiguous())
        desc1 = self.matcher.input_proj(descriptors1.contiguous())
        encoding0 = self.matcher.posenc(keypoints0)
        encoding1 = self.matcher.posenc(keypoints1)

        for transformer in self.matcher.transformers:
            desc0, desc1 = exportable_transformer_layer(transformer, desc0, desc1, encoding0, encoding1, valid0, valid1)

        scores = exportable_assignment_scores(self.matcher.log_assignment[-1], desc0, desc1, valid0, valid1)
        matches0, matches1, match_scores0, match_scores1 = filter_matches(scores, self.min_confidence)
        matches0, match_scores0 = self._mask_matches(matches0, match_scores0, valid0, valid1)
        matches1, match_scores1 = self._mask_matches(matches1, match_scores1, valid1, valid0)

        return matches0.to(dtype=torch.int32), matches1.to(dtype=torch.int32), match_scores0, match_scores1

    @staticmethod
    def _mask_matches(matches: Tensor, scores: Tensor, valid_source: Tensor, valid_target: Tensor) -> tuple[Tensor, Tensor]:
        safe_matches = matches.clamp(min=0)
        target_valid = valid_target.gather(1, safe_matches)
        valid_match = valid_source & (matches >= 0) & target_valid
        valid_score = valid_source & ((matches < 0) | target_valid)
        return torch.where(valid_match, matches, matches.new_tensor(-1)), torch.where(valid_score, scores, scores.new_tensor(0.0))


def dynamic_batch_axes(input_names: list[str], output_names: list[str]) -> dict[str, dict[int, str]]:
    return {name: {0: "batch_size"} for name in [*input_names, *output_names]}


@click.command(context_settings={"help_option_names": ["-h", "--help"]})
@click.argument("export-path", type=click.Path(path_type=Path))
@click.option(
    "--weights",
    "weights_path",
    type=click.Path(exists=True, path_type=Path),
    default=None,
    help="Path to the XFeat LighterGlue .pt weights. Defaults to the accelerated-features package weights.",
)
@click.option("--keypoints", "keypoint_count", default=512, show_default=True, help="Fixed keypoint count.")
@click.option("--min-confidence", default=0.1, show_default=True, help="Minimum match confidence.")
@click.option("--opset", default=20, show_default=True, help="ONNX opset version.")
@click.option("--device", default="cpu", show_default=True, help="Torch export device, e.g. cpu or cuda:0.")
@click.option("--dynamic-batch", is_flag=True, help="Mark batch axes dynamic.")
def main(
    export_path: Path,
    *,
    weights_path: Path | None,
    keypoint_count: int,
    min_confidence: float,
    opset: int,
    device: str,
    dynamic_batch: bool,
) -> None:
    if keypoint_count <= 0:
        raise click.BadParameter("--keypoints must be > 0")

    wrapper = LighterGlueFixedWrapper(weights_path or default_weights_path(), min_confidence=min_confidence).to(device)
    wrapper.eval()

    keypoints = torch.zeros((1, keypoint_count, 2), dtype=torch.float32, device=device)
    descriptors = torch.zeros((1, keypoint_count, 64), dtype=torch.float32, device=device)
    valid = torch.ones((1, keypoint_count), dtype=torch.bool, device=device)

    input_names = [
        "keypoints0",
        "keypoints1",
        "descriptors0",
        "descriptors1",
        "valid0",
        "valid1",
    ]
    output_names = ["matches0", "matches1", "matching_scores0", "matching_scores1"]

    export_path.parent.mkdir(parents=True, exist_ok=True)
    torch.onnx.export(
        wrapper,
        (keypoints, keypoints, descriptors, descriptors, valid, valid),
        export_path,
        input_names=input_names,
        output_names=output_names,
        dynamic_axes=dynamic_batch_axes(input_names, output_names) if dynamic_batch else None,
        opset_version=opset,
        external_data=False,
        dynamo=False,
    )

    click.echo(f"Exported LighterGlue ONNX model to: {os.path.abspath(export_path)}")


if __name__ == "__main__":
    main()
