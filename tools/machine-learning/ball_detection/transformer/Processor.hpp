#pragma once

#include <filesystem>
#include <fstream>
#include <map>
#include <vector>

#include "Configuration.hpp"
#include "TFRecordOffset.hpp"

namespace Hulks::Transformer
{

  class Processor
  {
  public:
    explicit Processor(Configuration configuration);
    void runUntilComplete();

  private:
    bool discoverWeightedInputTFRecordPath(
        const std::vector<Configuration::WeightedTFRecordPath>::const_iterator&
            weightedInputTFRecordPathIterator);
    bool discoverTFRecordPath(
        const std::vector<Configuration::WeightedTFRecordPath>::const_iterator&
            weightedInputTFRecordPathIterator,
        const std::filesystem::path& tfRecordPath);
    void collectTFRecordPaths();
    bool writeTFRecords();

    Configuration configuration_;

    std::map<std::vector<Configuration::WeightedTFRecordPath>::const_iterator,
             std::vector<TFRecordOffset>>
        discoveredTFRecords_;
    std::vector<TFRecordOffset> collectedTFRecordOffsets_;
  };

} // namespace Hulks::Transformer
