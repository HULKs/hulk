import csv
import json


def convert_json_to_csv(json_path, csv_path):
    with open(json_path) as jf:
        data = json.load(jf)

    trials = data["trials"]
    rows = zip(trials["f_score"], trials["n_features"], trials["trial_number"])

    with open(csv_path, "w", newline="") as cf:
        writer = csv.writer(cf)
        writer.writerow(["f_score", "n_features", "trial_number"])  # Header
        writer.writerows(rows)


def extract_best_to_csv(json_path, csv_path):
    with open(json_path) as jf:
        data = json.load(jf)

    best = data["best"]
    with open(csv_path, "w", newline="") as cf:
        writer = csv.writer(cf)
        writer.writerow(["f_score", "n_features", "best"])  # best=1 for display
        writer.writerow([best["f_score"], best["n_features"], 1])


convert_json_to_csv(
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/poster/data/testVal_DecisionTree.json",
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/paper/data/testVal_DecisionTree_trials.csv",
)
extract_best_to_csv(
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/poster/data/testVal_DecisionTree.json",
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/paper/data/testVal_DecisionTree_best.csv",
)


convert_json_to_csv(
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/poster/data/testVal_LinearSVM.json",
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/paper/data/testVal_LinearSVM_trials.csv",
)
extract_best_to_csv(
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/poster/data/testVal_LinearSVM.json",
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/paper/data/testVal_LinearSVM_best.csv",
)


convert_json_to_csv(
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/poster/data/testVal_NystroemPolynomial.json",
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/paper/data/testVal_NystroemPolynomial_trials.csv",
)
extract_best_to_csv(
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/poster/data/testVal_NystroemPolynomial.json",
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/paper/data/testVal_NystroemPolynomial_best.csv",
)


convert_json_to_csv(
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/poster/data/testVal_NystroemRBF.json",
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/paper/data/testVal_NystroemRBF_trials.csv",
)
extract_best_to_csv(
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/poster/data/testVal_NystroemRBF.json",
    "/home/franziska-sophie/Documents/2025-franziska-sophie-goettsch-project-work/paper/data/testVal_NystroemRBF_best.csv",
)
