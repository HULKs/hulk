#pragma once

namespace Hulks::GridCropper
{

  struct EvaluationResult
  {
    bool isPositive{false};
    float ballConfidence{0.f};
    float positionX{0.f};
    float positionY{0.f};
    float radius{0.f};
  };

} // namespace Hulks::GridCropper
