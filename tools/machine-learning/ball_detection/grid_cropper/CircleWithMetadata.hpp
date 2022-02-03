#pragma once

#include "Circle.hpp"
#include "EvaluationResult.hpp"

namespace Hulks::GridCropper
{

  struct CircleWithMetadata
  {
    Circle<float> sampleCircle{};
    Circle<float> correctedCircle{};
    EvaluationResult evaluationResult;

    CircleWithMetadata(Circle<float> sampleCircle, Circle<float> correctedCircle,
                       EvaluationResult evaluationResult)
      : sampleCircle{sampleCircle}
      , correctedCircle{correctedCircle}
      , evaluationResult{evaluationResult}
    {
    }
  };

} // namespace Hulks::GridCropper
