#include <algorithm>
#include <cxxopts.hpp>
#include <filesystem>
#include <iostream>
#include <map>
#include <runner.hpp>
#include <string>
#include <utility>

#include "Configuration.hpp"
#include "Processor.hpp"
#include "extract.hpp"

int main(int argc, char* argv[])
{
  cxxopts::Options options{"grid-cropping",
                           "Generates annotations based on cropping images on a grid"};

  Hulks::GridCropper::Configuration configuration;

  options.add_option("", "h", "help", "Print help", cxxopts::value<bool>(), "");

  options.add_option("", "", "maximum-square-size-factor",
                     "This factor * image.height will be the first squareSize",
                     cxxopts::value<decltype(configuration.maximumSquareSizeFactor)>(
                         configuration.maximumSquareSizeFactor)
                         ->default_value(std::to_string(configuration.maximumSquareSizeFactor)),
                     "factor");
  options.add_option(
      "", "", "minimum-square-size", "The minimum size of the square",
      cxxopts::value<decltype(configuration.minimumSquareSize)>(configuration.minimumSquareSize)
          ->default_value(std::to_string(configuration.minimumSquareSize)),
      "size");
  options.add_option(
      "", "", "square-size-step", "The linear decrease while iterating",
      cxxopts::value<decltype(configuration.squareSizeStep)>(configuration.squareSizeStep)
          ->default_value(std::to_string(configuration.squareSizeStep)),
      "step");
  options.add_option("", "", "sample-size", "The size of the image to crop",
                     cxxopts::value<decltype(configuration.sampleSize)>(configuration.sampleSize)
                         ->default_value(std::to_string(configuration.sampleSize)),
                     "size");
  options.add_option("", "", "ball-confidence-threshold",
                     "The threshold of confidence to detect a ball",
                     cxxopts::value<decltype(configuration.ballConfidenceThreshold)>(
                         configuration.ballConfidenceThreshold)
                         ->default_value(std::to_string(configuration.ballConfidenceThreshold)),
                     "threshold");
  options.add_option(
      "", "", "default-color",
      "The color used for pixels outside of image (float Y component in [0,255])",
      cxxopts::value<decltype(configuration.defaultColor)>(configuration.defaultColor)
          ->default_value(std::to_string(configuration.defaultColor)),
      "y");
  options.add_option(
      "", "", "merge-radius-factor", "The radius factor for clustering accepted candidates",
      cxxopts::value<decltype(configuration.mergeRadiusFactor)>(configuration.mergeRadiusFactor)
          ->default_value(std::to_string(configuration.mergeRadiusFactor)),
      "factor");

  std::map<std::string, Hulks::GridCropper::Configuration::ColorSpace> colorSpaceMapping{
      {"ycbcr", Hulks::GridCropper::Configuration::ColorSpace::YCBCR},
      {"rgb", Hulks::GridCropper::Configuration::ColorSpace::RGB},
      {"grayscale", Hulks::GridCropper::Configuration::ColorSpace::GRAYSCALE}};
  std::string colorSpace{std::find_if(colorSpaceMapping.begin(), colorSpaceMapping.end(),
                                      [&configuration](const auto& item) {
                                        return item.second == configuration.colorSpace;
                                      })
                             ->first};
  options.add_option(
      "", "", "color-space", "The color space of the images, one of: ycbcr, rgb, grayscale",
      cxxopts::value<decltype(colorSpace)>(colorSpace)->default_value(colorSpace), "color-space");

  options.add_option("", "", "confidence-factor-weight", "Weight of confidence factor",
                     cxxopts::value<decltype(configuration.confidenceFactorWeight)>(
                         configuration.confidenceFactorWeight)
                         ->default_value(std::to_string(configuration.confidenceFactorWeight)),
                     "factor");
  options.add_option(
      "", "", "correction-proximity-factor-weight", "Weight of correction proximity factor",
      cxxopts::value<decltype(configuration.correctionProximityFactorWeight)>(
          configuration.correctionProximityFactorWeight)
          ->default_value(std::to_string(configuration.correctionProximityFactorWeight)),
      "factor");
  options.add_option(
      "", "", "image-containment-factor-weight", "Weight of image containment factor",
      cxxopts::value<decltype(configuration.imageContainmentFactorWeight)>(
          configuration.imageContainmentFactorWeight)
          ->default_value(std::to_string(configuration.imageContainmentFactorWeight)),
      "factor");

  options.add_option("", "", "OUTPUT_ANNOTATIONS_FILE", "The path of the output annotations file",
                     cxxopts::value<decltype(configuration.outputAnnotationsFile)>(
                         configuration.outputAnnotationsFile),
                     "");
  options.add_option("", "", "DATA_DIRECTORIES_OR_FILES",
                     "The directories containing images or single image files",
                     cxxopts::value<decltype(configuration.dataDirectoriesOrFiles)>(
                         configuration.dataDirectoriesOrFiles),
                     "");
  options.parse_positional({"OUTPUT_ANNOTATIONS_FILE", "DATA_DIRECTORIES_OR_FILES"});
  options.positional_help("OUTPUT_ANNOTATIONS_FILE DATA_DIRECTORIES_OR_FILES...");

  const auto result = options.parse(argc, argv);

  if (result.count("help") != 0)
  {
    std::cout << options.help() << '\n';
    return 0;
  }

  if (result.count("color-space") != 0)
  {
    try
    {
      configuration.colorSpace = colorSpaceMapping.at(colorSpace);
    }
    catch (const std::out_of_range&)
    {
      std::cerr << "Unexpected color space: " << colorSpace << '\n';
      std::cout << options.help() << '\n';
      return 1;
    }
  }

  configuration.classifierModelPath = Hulks::GridCropper::extractClassifier();
  configuration.positionerModelPath = Hulks::GridCropper::extractPositioner();

  Hulks::Runner::Runner<Hulks::GridCropper::Processor> runner;
  runner.runUntilComplete(configuration);

  return 0;
}
