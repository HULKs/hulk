# Data Inventory & Format Report

Generated from full-dataset validation (not sampling) on 2026-05-31.
Location: `C:\Projects\RF-DETR\data\`

## Executive summary

Three subsets were dropped in. They differ in **format**, **resolution**, **class taxonomy**, and — most importantly — **visual domain**.

| Subset | Images | Labels | Format | Resolution | Domain | Verdict |
|---|---|---|---|---|---|---|
| `nao_dataset` | 16,644 (13,106 train / 3,538 val) | YOLO detection, 1:1 | **YOLO detection** (`class cx cy w h`, normalized) | 640×480 PNG (4:3) | RoboCup robot-eye (indoor field) | ✅ **PRIMARY training corpus** |
| `football-field-keypoints-dataset` | 342 (262 / 80) | YOLO detection (fixed 20×20px boxes) | **YOLO detection** (landmarks-as-boxes) | 1280×720 JPG (16:9) | **Broadcast TV soccer (FIFA World Cup)** | ❌ Wrong domain — exclude |
| `htwk_T1` (GP1_RoboErectus_Salvador_2025...) | 16,926 | **NONE** | unlabeled raw frames | 1280×800 JPG (16:10) | Booster K1 robot-eye (RoboCup 2025) | ⚠️ Deployment domain — needs labeling |

**Answer to the headline question:** the NAO dataset is in **YOLO detection format** (Ultralytics-style): one `.txt` per image, each line `class_id center_x center_y width height`, all coordinates normalized to [0,1]; class names in `nao_data.yaml`. It is **not** keypoint/pose, **not** segmentation, **not** COCO.

---

## Subset 1 — `nao_dataset` (PRIMARY)

- **Format**: YOLO detection. Validated: **all 46,512 train + 12,479 val annotation lines have exactly 5 fields** (zero pose/segmentation contamination). **Zero** out-of-[0,1] coordinate values. Perfect 1:1 image↔label pairing (0 orphans in either split).
- **Images**: 640×480 RGB PNG — classic NAO top-camera resolution.
- **Filenames**: UUIDs (e.g. `34276524-b01a-...png`), label stem matches image stem.
- **7 classes** (`nao_data.yaml`): `0:Ball  1:GoalPost  2:LSpot  3:PenaltySpot  4:Robot  5:TSpot  6:XSpot`
  - L/T/X-Spot = field-line junction types (L-junction, T-junction, X-crossing) — used for self-localization.
- **Class distribution** (train instances): Robot 16,914 · LSpot 8,999 · TSpot 5,637 · XSpot 4,638 · GoalPost 4,323 · Ball 3,376 · PenaltySpot 2,625. (val mirrors this.)
- **Background images**: 4,341/13,106 train (33%) and 1,112/3,538 val (31%) have **empty** label files. Normal for RF-DETR (negatives improve precision), but the high fraction is worth a sanity look — confirm these are intentional negatives, not unlabeled frames.

## Subset 2 — `football-field-keypoints-dataset` (EXCLUDE)

- **Format**: YOLO detection, all 3,638 train + 1,109 val lines exactly 5 fields. **Every box is identical size** (0.015625 × 0.027778 ≈ 20×20 px at 1280×720) → point-landmarks encoded as tiny boxes, one class per landmark.
- **Images**: 1280×720 RGB JPG (16:9 broadcast aspect).
- **28 classes** (`football-field-keypoints-data.yaml`): `HLC HRC HLGB HRGB ...` — full-pitch landmark scheme (goal box, goal line, penalty area, center circle, halfway line) for camera→pitch homography.
- **Domain (confirmed visually)**: aerial **broadcast footage of a real FIFA World Cup match** — stadium crowd, Hisense/VISA/Budweiser ad boards, human players. This is **not** robot soccer. Class 16/17 are nearly empty (8/11 instances).
- **Verdict**: domain mismatch is severe (camera height, scale, optics, players vs robots). Including it would inject out-of-distribution data. **Recommend excluding from training.** Keep only as a reference for the "landmark detection for localization" idea.

## Subset 3 — `htwk_T1` (UNLABELED — deployment domain)

- **16,926 JPG frames**, 1280×800 RGB (16:10) — **the Booster K1 onboard camera resolution / deployment domain**.
- **Zero label files.** Matches Alex's note: HTWK K1 images, unlabeled.
- Source: RoboCup 2025 game vs RoboErectus (venue signage: "World Humanoid Robot Games").
- `htwk_T1_01.yaml` defines an *intended* 28-class landmark taxonomy (`TLC TRC BLC BRC TL6MC ...` = Top/Bottom-Left/Right corners, 6m/18m box corners & lines, arcs, midline/center) — but **no labels exist yet**.
- **Verdict**: highest-value data for closing the sim/NAO→K1 domain gap, but requires labeling before supervised use. Candidate for auto-labeling with the Phase-1 detection model, then human correction.

---

## Cross-cutting implications for the multi-task plan

1. **Task coverage from current data:**
   - **Detection — READY.** `nao_dataset` (7 classes, 16.6k imgs, clean YOLO).
   - **Segmentation — NO DATA.** No polygon/RLE masks anywhere. Phase 2 is blocked until masks are created.
   - **Pose/keypoints (COCO triplets) — NO DATA.** The "keypoints" subset is detection-of-landmarks, wrong domain. The PR #521 pose track has no training data in this drop.
   - **Field-landmark localization — partially READY** via NAO's L/T/X/Penalty spots (it's just detection), expandable by labeling `htwk_T1`.

2. **Domain & aspect-ratio shift is real and measurable:** train domain (NAO, 640×480, 4:3) ≠ deployment (K1, 1280×800, 16:10). At our 448×448 square training resolution, both need letterboxing; the aspect-ratio change between train and deploy is a concrete sim-to-real risk to monitor.

3. **No team-color robot split:** labels have a single `Robot` class (not red/blue). Our earlier guessed taxonomy (`robot_red`/`robot_blue`) does **not** exist in the data — corrected in `config/detection.yaml`.

4. **RF-DETR ingestion path:** `nao_dataset` is already YOLO, which RF-DETR can sometimes auto-detect — but its layout (`images/train`, `labels/train`, split name `val`) differs from RF-DETR's expected `train/`,`valid/` COCO layout. Converting YOLO→COCO is the robust path (also future-proofs for seg/pose, enables clean class remapping). This defines `src/convert_nao_to_rfdetr.py`.

---

## Recommended data strategy

- **Phase 1 (now):** Convert `nao_dataset` YOLO→COCO (`train/`, `valid/`), train RFDETRSmall detection on the 7 real classes. Keep background images (or subsample if 33% proves too high).
- **Exclude** `football-field-keypoints-dataset` from training (domain mismatch).
- **Phase 1.5 (domain adaptation):** auto-label a slice of `htwk_T1` with the Phase-1 model → human-correct → fine-tune. Closes the NAO→K1 gap.
- **Segmentation / Pose:** deferred — no data. Requires a dedicated annotation effort (or synthetic data from FS's Unity pipeline) before either task can start.

## Open decisions (for the user)

1. Keep all 7 NAO classes, or focus a subset (e.g., drop or merge L/T/X spots)?
2. Confirm exclusion of the broadcast-soccer keypoints set.
3. Background-image fraction (33%): keep all, subsample, or investigate?
4. Prioritize labeling `htwk_T1` (deployment domain) now, or after a NAO-only baseline exists?
