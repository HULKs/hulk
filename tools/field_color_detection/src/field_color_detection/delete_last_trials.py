import optuna


if __name__ == "__main__":
    ho_storage_name = "sqlite:///../../db.sqlite3"
    ho_study_name = "testVal_DecisionTree"

    study = optuna.create_study(
        directions=["maximize", "minimize"],
        storage=ho_storage_name,
        study_name=ho_study_name,
        load_if_exists=True,
    )
    num_trials_to_keep = 1500

    trials_to_keep = [
        optuna.trial.create_trial(
            state=t.state,
            params=t.params,
            user_attrs=t.user_attrs,
            system_attrs=t.system_attrs,
            intermediate_values=t.intermediate_values,
            distributions=t.distributions,
            values=t.values,
        )
        for t in study.trials
        if t.number < num_trials_to_keep
    ]

    # Delete study before recreating
    optuna.delete_study(study_name=ho_study_name, storage=ho_storage_name)

    # Recreate study and add trials to keep
    new_study = optuna.create_study(
        directions=["maximize", "minimize"],
        storage=ho_storage_name,
        study_name=ho_study_name,
        load_if_exists=True,
    )
    new_study.add_trials(trials_to_keep)
