#include <cerrno>
#include <cstdlib>
#include <cxxopts.hpp>
#include <iostream>
#include <memory>
#include <regex>
#include <string>

#include "Configuration.hpp"
#include "Processor.hpp"

int main(int argc, char* argv[])
{
  cxxopts::Options options{
      "transformer",
      "Creates merged TFRecords from sample TFRecords\n\nThe weighted paths must have "
      "the following format: \"path/to/my.tfrecord\" (unweighted) or \"0.5:path/to/my.tfrecord\" "
      "(weighted)\n"};

  Hulks::Transformer::Configuration configuration;

  options.add_option("", "h", "help", "Print help", cxxopts::value<bool>(), "");

  options.add_option(
      "", "", "shuffle-random-seed", "Random seed for shuffling",
      cxxopts::value<decltype(configuration.shuffleRandomSeed)>(configuration.shuffleRandomSeed)
          ->default_value(std::to_string(configuration.shuffleRandomSeed)),
      "seed");

  std::vector<std::string> weightedTFRecordPaths;
  options.add_option("", "", "WEIGHTED_TFRECORD_PATHS", "The paths of the input TFRecord files",
                     cxxopts::value<decltype(weightedTFRecordPaths)>(weightedTFRecordPaths), "");
  options.parse_positional({"WEIGHTED_TFRECORD_PATHS"});
  options.positional_help("WEIGHTED_INPUT_TFRECORD_PATHS... - WEIGHTED_OUTPUT_TFRECORD_PATHS...");

  const auto result = options.parse(argc, argv);

  if (result.count("help") != 0)
  {
    std::cout << options.help() << '\n';
    return 0;
  }

  bool iteratingOverInputPaths = true;
  float weightSum = 0.f;
  std::size_t amountOfUnweightedPaths = 0;
  for (const auto& weightedPath : weightedTFRecordPaths)
  {
    std::cout << weightedPath << '\n';
    // a "-" argument switches from inputs to outputs and continues
    if (weightedPath == "-")
    {
      iteratingOverInputPaths = false;
      continue;
    }

    if (iteratingOverInputPaths)
    {
      configuration.weightedInputTFRecordPaths.emplace_back(weightedPath);
      continue;
    }

    const auto& emplacedPath = configuration.weightedOutputTFRecordPaths.emplace_back(weightedPath);
    if (emplacedPath.considerWeight)
    {
      weightSum += emplacedPath.weight;
    }
    else
    {
      ++amountOfUnweightedPaths;
    }
  }

  // equally split remaining weight between outputs of unspecified weight
  if (weightSum < 1.f)
  {
    for (auto& outputPath : configuration.weightedOutputTFRecordPaths)
    {
      if (!outputPath.considerWeight)
      {
        outputPath.considerWeight = true;
        outputPath.weight = (1.f - weightSum) / static_cast<float>(amountOfUnweightedPaths);
      }
    }
  }

  Hulks::Transformer::Processor processor{configuration};
  processor.runUntilComplete();

  return 0;
}
