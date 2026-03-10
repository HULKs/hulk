import torch
from torch import ByteTensor, Tensor, nn


class NV12ToRgb(nn.Module):
    def __init__(self) -> None:
        super().__init__()
        self.yuv_to_rgb = nn.Parameter(
            torch.tensor(
                [
                    [1.0, 1.0, 1.0],
                    [0.0, -0.344136, 1.772],
                    [1.402, -0.714136, 0.0],
                ]
            )
            / 255.0,
            requires_grad=False,
        )
        self.yuv_to_rgb_offset = nn.Parameter(
            torch.tensor([0.0, 128.0, 128.0]), requires_grad=False
        )

    def forward(self, byte_data: ByteTensor) -> Tensor:
        image_data = byte_data.to(torch.float32)
        half_height, half_width, _ = image_data.shape
        height, width = half_height * 2, half_width * 2
        assert image_data.size(-1) == 6
        luminance = image_data.view(-1)[: width * height].view(height, width, 1)
        chroma_subsampled = image_data.view(-1)[width * height :].view(
            half_height, half_width, 2
        )
        yuv = torch.concat([luminance[::2, ::2], chroma_subsampled], -1)
        rgb = torch.matmul(yuv - self.yuv_to_rgb_offset, self.yuv_to_rgb)
        return rgb


if __name__ == "__main__":
    import json
    from pathlib import Path

    import matplotlib.pyplot as plt

    data = json.loads(Path("image.json").read_text())
    byte_tensor = torch.tensor(data["data"], dtype=torch.uint8)
    width = data["width"]
    height = data["height"]

    converter = NV12ToRgb()
    rgb_tensor = converter(byte_tensor.view(height // 2, width // 2, 6))
    plt.imshow(rgb_tensor)
    plt.savefig("output.png")
