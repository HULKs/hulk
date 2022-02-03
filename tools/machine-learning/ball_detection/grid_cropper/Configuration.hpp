#pragma once

#include <filesystem>
#include <vector>

namespace Hulks::GridCropper
{

  struct Configuration
  {
    enum class ColorSpace
    {
      YCBCR,
      RGB,
      GRAYSCALE,
    };

    std::filesystem::path classifierModelPath{};
    std::filesystem::path positionerModelPath{};

    float maximumSquareSizeFactor{0.5f};
    int minimumSquareSize{7};
    int squareSizeStep{10};
    int sampleSize{32};
    float ballConfidenceThreshold{0.9f};
    float defaultColor{128.f};
    float mergeRadiusFactor{1.5f};
    ColorSpace colorSpace{ColorSpace::YCBCR};
    float confidenceFactorWeight{2.f};
    float correctionProximityFactorWeight{0.5f};
    float imageContainmentFactorWeight{0.5f};

    std::filesystem::path outputAnnotationsFile{};
    std::vector<std::filesystem::path> dataDirectoriesOrFiles{};
  };

} // namespace Hulks::GridCropper
