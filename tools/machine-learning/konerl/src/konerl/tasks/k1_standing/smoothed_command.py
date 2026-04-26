import torch
import typing
from dataclasses import dataclass
from mjlab.envs.manager_based_rl_env import ManagerBasedRlEnv
from mjlab.tasks.velocity.mdp import UniformVelocityCommand, UniformVelocityCommandCfg
from mjlab.utils.lab_api.math import quat_apply

@dataclass(kw_only=True)
class SmoothedVelocityCommandCfg(UniformVelocityCommandCfg):
    ema_alpha: float = 0.05

    @dataclass
    class SmoothRanges:
        lin_vel_x: tuple[float, float]
        lin_vel_y: tuple[float, float]
        ang_vel_z: tuple[float, float]
        heading: tuple[float, float] | None = None

    ranges: SmoothRanges # type: ignore

    def build(self, env: ManagerBasedRlEnv) -> "SmoothedVelocityCommand":
        return SmoothedVelocityCommand(self, env)

class SmoothedVelocityCommand(UniformVelocityCommand):
    cfg: SmoothedVelocityCommandCfg # type: ignore

    def __init__(self, cfg: SmoothedVelocityCommandCfg, env: ManagerBasedRlEnv):
        super().__init__(cfg, env)
        self.vel_command_b = torch.zeros(self.num_envs, 3, device=self.device)
        self.target_vel_command_b = torch.zeros_like(self.vel_command_b)
        self.target_heading_b = torch.zeros(self.num_envs, device=self.device)

    def _resample_command(self, env_ids: torch.Tensor) -> None:
        num_resample = len(env_ids)
        r = torch.empty(num_resample, device=self.device)
        
        x_max, x_min = self.cfg.ranges.lin_vel_x[1], abs(self.cfg.ranges.lin_vel_x[0])
        y_max, y_min = self.cfg.ranges.lin_vel_y[1], abs(self.cfg.ranges.lin_vel_y[0])
        z_max, z_min = self.cfg.ranges.ang_vel_z[1], abs(self.cfg.ranges.ang_vel_z[0])

        # 1. Density Fix: Generate absolute normal, apply volume-weighted signs
        u = torch.abs(torch.randn(num_resample, 3, device=self.device))
        
        r_x = torch.rand(num_resample, device=self.device)
        u[:, 0] *= torch.where(r_x < (x_max / (x_max + x_min + 1e-6)), 1.0, -1.0)
        
        r_y = torch.rand(num_resample, device=self.device)
        u[:, 1] *= torch.where(r_y < (y_max / (y_max + y_min + 1e-6)), 1.0, -1.0)
        
        r_z = torch.rand(num_resample, device=self.device)
        u[:, 2] *= torch.where(r_z < (z_max / (z_max + z_min + 1e-6)), 1.0, -1.0)

        u_norm = u / torch.clamp_min(torch.norm(u, dim=1, keepdim=True), 1e-6)

        radius = torch.rand(num_resample, 1, device=self.device).pow(1.0 / 5.0)
        sphere_points = u_norm * radius

        self.target_vel_command_b[env_ids, 0] = torch.where(
            sphere_points[:, 0] > 0, sphere_points[:, 0] * x_max, sphere_points[:, 0] * x_min
        )
        self.target_vel_command_b[env_ids, 1] = torch.where(
            sphere_points[:, 1] > 0, sphere_points[:, 1] * y_max, sphere_points[:, 1] * y_min
        )
        self.target_vel_command_b[env_ids, 2] = torch.where(
            sphere_points[:, 2] > 0, sphere_points[:, 2] * z_max, sphere_points[:, 2] * z_min
        )

        self.target_heading_b[env_ids] = self.robot.data.heading_w[env_ids]
        self.is_standing_env[env_ids] = r.uniform_(0.0, 1.0) <= self.cfg.rel_standing_envs

        init_vel_mask = r.uniform_(0.0, 1.0) < self.cfg.init_velocity_prob
        init_vel_env_ids = env_ids[init_vel_mask]
        
        if len(init_vel_env_ids) > 0:
            root_pos = self.robot.data.root_link_pos_w[init_vel_env_ids]
            root_quat = self.robot.data.root_link_quat_w[init_vel_env_ids]
            lin_vel_b = self.robot.data.root_link_lin_vel_b[init_vel_env_ids]
            
            lin_vel_b[:, :2] = self.target_vel_command_b[init_vel_env_ids, :2]
            root_lin_vel_w = quat_apply(root_quat, lin_vel_b)
            
            root_ang_vel_b = self.robot.data.root_link_ang_vel_b[init_vel_env_ids]
            root_ang_vel_b[:, 2] = self.target_vel_command_b[init_vel_env_ids, 2]
            
            root_state = torch.cat(
                [root_pos, root_quat, root_lin_vel_w, root_ang_vel_b], dim=-1
            )
            self.robot.write_root_state_to_sim(root_state, init_vel_env_ids)
            
            self.vel_command_b[init_vel_env_ids] = self.target_vel_command_b[init_vel_env_ids]

    def _update_command(self) -> None:
        cfg = typing.cast(SmoothedVelocityCommandCfg, self.cfg)

        standing_env_ids = self.is_standing_env.nonzero(as_tuple=False).flatten()
        self.target_vel_command_b[standing_env_ids, :] = 0.0

        self.vel_command_b = (
            cfg.ema_alpha * self.target_vel_command_b +
            (1.0 - cfg.ema_alpha) * self.vel_command_b
        )
        
        self.vel_command_b[:, 2] = torch.clip(
            self.vel_command_b[:, 2],
            min=cfg.ranges.ang_vel_z[0],
            max=cfg.ranges.ang_vel_z[1],
        )

        self.vel_command_b[standing_env_ids, :] = 0.0

    @property
    def command(self) -> torch.Tensor:
        return self.vel_command_b