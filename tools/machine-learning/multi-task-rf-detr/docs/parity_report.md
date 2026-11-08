# ONNX Hand-off Contract

- File: `rf-detr-trial-448.onnx`  (120.5 MB)
- Variant: RFDETRSmall @ 448^2, static batch=True
- onnxruntime 1.26.0; providers: ['TensorrtExecutionProvider', 'CUDAExecutionProvider', 'CPUExecutionProvider']; CUDA EP bound: True
- Validation input: 0002e546-9a7c-4996-af28-eafedd8123f0.png

## Classes  (ONNX `labels` has num_classes+1 columns; last column = no-object)
  0: Ball
  1: GoalPost
  2: LSpot
  3: PenaltySpot
  4: Robot
  5: TSpot
  6: XSpot

## Inputs
  input: [1, 3, 448, 448] tensor(float)

## Outputs
  dets: [1, 300, 4] tensor(float)
  labels: [1, 300, 8] tensor(float)

  dets = (cx,cy,w,h) normalized per query; labels = class logits (apply sigmoid).

## Checks
  CPU outputs finite: True
  CUDA outputs finite: True
  Output shapes match graph: True
  CPU vs CUDA(fp32) max abs diff = 6.676e-05 (tol 1e-03) -> PASS

## Robot-side preprocessing
  - NCHW float32, ImageNet mean/std, letterbox to square resolution.
  - Build TensorRT engine with fixed 1x3xRxR input (static batch).
  - NOTE: deployment uses FP16 TensorRT; accuracy parity vs PyTorch is validated
    on-robot, not here. This report certifies the graph is valid and runnable.