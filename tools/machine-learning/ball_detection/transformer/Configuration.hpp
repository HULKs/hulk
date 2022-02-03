#pragma once

#include <filesystem>
#include <string>
#include <vector>

namespace Hulks::Transformer
{

  struct Configuration
  {
    struct WeightedTFRecordPath
    {
      bool considerWeight{false};
      float weight{0.f};
      std::filesystem::path path;

      WeightedTFRecordPath(bool considerWeight, float weight, std::filesystem::path path);
      explicit WeightedTFRecordPath(const std::string& argument);
    };

    uint32_t shuffleRandomSeed{42};

    std::vector<WeightedTFRecordPath> weightedInputTFRecordPaths;
    std::vector<WeightedTFRecordPath> weightedOutputTFRecordPaths;

  };

} // namespace Hulks::Transformer
