from mjlab.rl import (
    RslRlOnPolicyRunnerCfg,
    RslRlModelCfg,
    RslRlPpoAlgorithmCfg,
)


def k1_ppo_runner_cfg() -> RslRlOnPolicyRunnerCfg:
    return RslRlOnPolicyRunnerCfg(
        actor=RslRlModelCfg(
            obs_normalization=True,
            hidden_dims=(512, 256, 128),
            activation="elu",
            distribution_cfg={
                "class_name": "GaussianDistribution",
                "init_std": 0.5,
                "std_type": "log",
            },
        ),
        critic=RslRlModelCfg(
            obs_normalization=True,
            hidden_dims=(512, 256, 128),
            activation="elu",
            distribution_cfg={
                "class_name": "GaussianDistribution",
                "init_std": 0.5,
                "std_type": "log",
            },
        ),
        algorithm=RslRlPpoAlgorithmCfg(
            value_loss_coef=1.0,
            use_clipped_value_loss=True,
            clip_param=0.2,
            entropy_coef=0.01,
            num_learning_epochs=4,
            num_mini_batches=16,
            learning_rate=3.0e-4,
            schedule="adaptive",
            gamma=0.995,
            lam=0.95,
            desired_kl=0.01,
            max_grad_norm=1.0,
        ),
        experiment_name="k1_velocity_tracking",
        save_interval=200,
        num_steps_per_env=48,
        max_iterations=10_000,
        clip_actions=5.0,
    )
