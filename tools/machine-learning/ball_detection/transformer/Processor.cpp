#include "Processor.hpp"

#include <algorithm>
#include <boost/crc.hpp>
#include <cassert>
#include <chrono>
#include <cmath>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <random>
#include <ratio>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

namespace Hulks::Transformer
{

  // CRC32-C (Castagnoli)
  using crc_32c_type = boost::crc_optimal<32, 0x1EDC6F41, 0xFFFFFFFF, 0xFFFFFFFF, true, true>;
  static const uint32_t K_MASK_DELTA = 0xa282ead8ul;

  Processor::Processor(Configuration configuration)
    : configuration_{std::move(configuration)}
  {
  }

  void Processor::runUntilComplete()
  {
    if (configuration_.weightedOutputTFRecordPaths.empty())
    {
      std::cerr << "Cannot run with no output files.\n";
      return;
    }

    for (auto weightedInputTFRecordPathIterator =
             configuration_.weightedInputTFRecordPaths.cbegin();
         weightedInputTFRecordPathIterator != configuration_.weightedInputTFRecordPaths.cend();
         ++weightedInputTFRecordPathIterator)
    {
      if (!discoverWeightedInputTFRecordPath(weightedInputTFRecordPathIterator))
      {
        std::cerr << "failed to collect input path "
                  << weightedInputTFRecordPathIterator->path.string() << '\n';
        return;
      }
    }

    collectTFRecordPaths();

    writeTFRecords();
  }

  bool Processor::discoverWeightedInputTFRecordPath(
      const std::vector<Configuration::WeightedTFRecordPath>::const_iterator&
          weightedInputTFRecordPathIterator)
  {
    std::error_code errorCode;
    if (std::filesystem::is_regular_file(weightedInputTFRecordPathIterator->path, errorCode))
    {
      if (weightedInputTFRecordPathIterator->path.extension() == ".tfrecord" &&
          !discoverTFRecordPath(weightedInputTFRecordPathIterator,
                                weightedInputTFRecordPathIterator->path))
      {
        std::cerr << "failed to collect path " << weightedInputTFRecordPathIterator->path.string()
                  << '\n';
        return false;
      }

      return true;
    }

    if (!errorCode &&
        std::filesystem::is_directory(weightedInputTFRecordPathIterator->path, errorCode))
    {
      for (const auto& tfRecordPath : std::filesystem::recursive_directory_iterator{
               weightedInputTFRecordPathIterator->path, errorCode})
      {
        if (tfRecordPath.path().extension() == ".tfrecord" &&
            !discoverTFRecordPath(weightedInputTFRecordPathIterator, tfRecordPath.path()))
        {
          std::cerr << "failed to collect path " << tfRecordPath.path().string() << '\n';
          return false;
        }
      }

      if (errorCode)
      {
        std::cerr << "directory " << weightedInputTFRecordPathIterator->path
                  << " iteration failed: " << errorCode.message() << '\n';
        return false;
      }

      return true;
    }

    std::cerr << "file " << weightedInputTFRecordPathIterator->path
              << " stat retrieval failed: " << errorCode.message() << '\n';
    return false;
  }

  bool Processor::discoverTFRecordPath(
      const std::vector<Configuration::WeightedTFRecordPath>::const_iterator&
          weightedInputTFRecordPathIterator,
      const std::filesystem::path& tfRecordPath)
  {
    std::ifstream tfRecordFile{tfRecordPath};
    if (!tfRecordFile.is_open())
    {
      std::cerr << "failed to open tfrecord " << tfRecordPath.string() << '\n';
      return false;
    }

    // get file size
    tfRecordFile.seekg(0, std::ios_base::end);
    std::ifstream::pos_type size = tfRecordFile.tellg();
    tfRecordFile.seekg(0, std::ios_base::beg);

    // generate offset map
    for (std::ifstream::pos_type position = 0; position < size;)
    {
      tfRecordFile.seekg(position);

      // read and validate length
      uint64_t length = 0;
      tfRecordFile.read(reinterpret_cast<char*>(&length), sizeof(length));
      uint32_t maskedCRC32OfLength = 0;
      tfRecordFile.read(reinterpret_cast<char*>(&maskedCRC32OfLength), sizeof(maskedCRC32OfLength));
      if (!tfRecordFile.good())
      {
        std::cerr << "length reading failed in " << tfRecordPath.string() << '\n';
        return false;
      }

      // NOLINTNEXTLINE(readability-identifier-naming)
      crc_32c_type crc_32c;
      crc_32c.process_bytes(reinterpret_cast<const void*>(&length), sizeof(length));
      uint32_t checksum = crc_32c.checksum();
      if (maskedCRC32OfLength != ((checksum >> 15u) | (checksum << 17u)) + K_MASK_DELTA)
      {
        std::cerr << "CRC integrity check failed for length in " << tfRecordPath.string() << '\n';
        return false;
      }

      // validate next position (last position must equal size)
      if (position + static_cast<std::ifstream::pos_type>(length) +
              static_cast<std::ifstream::pos_type>(16) >
          size)
      {
        std::cerr << "data truncated in " << tfRecordPath.string() << ", ignoring last example\n";
        break;
      }

      // store position
      discoveredTFRecords_[weightedInputTFRecordPathIterator].emplace_back(
          tfRecordPath, position,
          static_cast<std::ifstream::pos_type>(length) + static_cast<std::ifstream::pos_type>(16));

      // update position:
      //   length + CRC(length) + data[length] + CRC(data[length])
      //   8      + 4           + length       + 4                 = length + 16
      position += length + 16;
    }

    return true;
  }

  void Processor::collectTFRecordPaths()
  {
    // shuffle discovered TFRecords, count weights and sizes
    std::mt19937 randomGenerator{configuration_.shuffleRandomSeed};
    float weightSum = 0.f;
    std::size_t sizeSum = 0;
    for (auto& [weightedInputTFRecordPathIterator, discovered] : discoveredTFRecords_)
    {
      std::shuffle(discovered.begin(), discovered.end(), randomGenerator);
      if (weightedInputTFRecordPathIterator->considerWeight)
      {
        weightSum += weightedInputTFRecordPathIterator->weight;
        sizeSum += discovered.size();
      }
    }

    // we calculate the input path which will be completely collected into the final dataset.
    // the calculation starts with the target weight ratios and discovered sizes.
    // we then calculate two ratios (weight and size) and divide the size ratio by the weight ratio.
    // the result will be large for small sizes with large weights, which dominate the TFRecord
    // collection.
    // the target ratio of the dominating input path induces the desired sizes of the other input
    // paths.
    std::map<std::vector<Configuration::WeightedTFRecordPath>::const_iterator, float>
        dominationScores;
    for (const auto& [weightedInputTFRecordPathIterator, discovered] : discoveredTFRecords_)
    {
      if (weightedInputTFRecordPathIterator->considerWeight)
      {
        dominationScores[weightedInputTFRecordPathIterator] =
            (weightedInputTFRecordPathIterator->weight / weightSum) /
            (static_cast<float>(discovered.size()) / static_cast<float>(sizeSum));
      }
    }

    if (!dominationScores.empty())
    {
      const auto dominating =
          std::max_element(dominationScores.begin(), dominationScores.end(),
                           [](const auto& a, const auto& b) { return a.second < b.second; })
              ->first;
      const auto dominatingWeight = dominating->weight;
      const auto dominatingSize = discoveredTFRecords_[dominating].size();
      const auto resultingSize =
          static_cast<float>(dominatingSize) / (dominatingWeight / weightSum);

      // collect the first TFRecord offsets with a given weight s.t. the ratios are achieved
      for (const auto& [weightedInputTFRecordPathIterator, discovered] : discoveredTFRecords_)
      {
        if (weightedInputTFRecordPathIterator->considerWeight)
        {
          const auto amountOfTFRecordOffsetsToCollect = static_cast<std::size_t>(
              resultingSize * (weightedInputTFRecordPathIterator->weight / weightSum));
          std::cout << "Appending " << amountOfTFRecordOffsetsToCollect << " of "
                    << weightedInputTFRecordPathIterator->path << '\n';
          assert(amountOfTFRecordOffsetsToCollect <= discovered.size());
          collectedTFRecordOffsets_.insert(collectedTFRecordOffsets_.end(), discovered.begin(),
                                           discovered.begin() + amountOfTFRecordOffsetsToCollect);
        }
      }
    }

    // collect all TFRecord offsets without a weight
    for (const auto& [weightedInputTFRecordPathIterator, discovered] : discoveredTFRecords_)
    {
      if (!weightedInputTFRecordPathIterator->considerWeight)
      {
        std::cout << "Appending all of " << weightedInputTFRecordPathIterator->path << '\n';
        collectedTFRecordOffsets_.insert(collectedTFRecordOffsets_.end(), discovered.begin(),
                                         discovered.end());
      }
    }

    std::shuffle(collectedTFRecordOffsets_.begin(), collectedTFRecordOffsets_.end(),
                 randomGenerator);
  }

  bool Processor::writeTFRecords()
  {
    using namespace std::chrono_literals;

    std::size_t done = 0;
    const auto total = collectedTFRecordOffsets_.size();

    std::vector<std::pair<std::size_t, std::ofstream>> outputTFRecords;

    // for each output path, open the file and store the running total of records
    std::size_t previousRunningTotal = 0;
    std::cout << "Output sample distribution:" << '\n';
    for (auto& outputPath : configuration_.weightedOutputTFRecordPaths)
    {
      const auto amountOfSamples =
          static_cast<std::size_t>(static_cast<float>(total) * outputPath.weight);
      const auto runningTotal = previousRunningTotal + amountOfSamples;
      previousRunningTotal = runningTotal;

      std::cout << "  " << outputPath.path.string() << ": " << amountOfSamples << '\n';

      const auto& outputTFRecord = outputTFRecords.emplace_back(
          runningTotal, std::ofstream(outputPath.path, std::ios_base::out | std::ios_base::trunc));
      if (!outputTFRecord.second.is_open())
      {
        std::cerr << "failed to open output file " << outputPath.path.string() << " for writing\n";
        return false;
      }
    }
    if (previousRunningTotal < total)
    {
      std::cout << "  Remainder: " << total - previousRunningTotal << '\n';
    }
    std::cout << '\n';

    bool beginInitialized = false;
    std::chrono::time_point<std::chrono::steady_clock> begin;
    std::chrono::time_point<std::chrono::steady_clock> lastOutput;

    std::vector<char> buffer;

    auto outputIterator = outputTFRecords.begin();

    for (const auto& offset : collectedTFRecordOffsets_)
    {
      if (!beginInitialized)
      {
        beginInitialized = true;
        begin = std::chrono::steady_clock::now();
      }

      // write TFRecord
      std::ifstream tfRecordFile{offset.path};
      if (!tfRecordFile.is_open())
      {
        std::cerr << "failed to open tfrecord " << offset.path.string() << '\n';
        return false;
      }

      tfRecordFile.seekg(offset.offset);

      buffer.resize(offset.length);
      tfRecordFile.read(buffer.data(), buffer.size());
      outputIterator->second.write(buffer.data(), buffer.size());

      ++done;
      if (done >= outputIterator->first)
      {
        ++outputIterator;
        if (outputIterator == outputTFRecords.end())
        {
          return true;
        }
      }

      // calculate elapsed time, interpolate remaining time and eventually output
      const auto current = std::chrono::steady_clock::now();
      if (current - lastOutput > 1s)
      {
        lastOutput = current;

        const auto totalLength = std::ceil(std::log10(total)) + 1;
        const auto totalDuration =
            std::chrono::duration_cast<std::chrono::duration<float>>(current - begin);
        const auto averageDuration = done == 0 ? 0s : totalDuration / done;
        const auto remainingEstimate = averageDuration * (total - done);

        std::cout
            << std::right << std::setw(totalLength) << done << "/" << total << ", " << std::right
            << std::setw(5) << std::setprecision(4)
            << std::chrono::duration_cast<std::chrono::duration<float>, float>(averageDuration)
                   .count()
            << "s each, " << std::setprecision(2)
            << std::chrono::duration_cast<std::chrono::duration<float, std::ratio<60>>, float>(
                   remainingEstimate)
                   .count()
            << "m remaining, current: " << offset.path.string() << "...\n";
      }
    }

    return true;
  }

} // namespace Hulks::Transformer
