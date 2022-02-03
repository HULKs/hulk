#include "Processor.hpp"

#include <algorithm>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <iterator>
#include <logger.hpp>
#include <memory>
#include <numeric>
#include <runner.hpp>
#include <set>
#include <string>
#include <utility>

#include "Circle.hpp"
#include "CircleWithMetadata.hpp"
#include "Cluster.hpp"
#include "Image.hpp"

#define STB_IMAGE_IMPLEMENTATION
#include <CompiledNN.h>
#include <Model.h>
#include <cmath>
#include <stb_image.h>

#define STB_IMAGE_RESIZE_IMPLEMENTATION
#include <stb_image_resize.h>

#include <nlohmann/json.hpp>

// NOLINTNEXTLINE(readability-identifier-naming)
namespace std
{

  template <>
  struct hash<std::filesystem::path>
  {
    std::size_t operator()(const std::filesystem::path& path) const noexcept
    {
      return std::filesystem::hash_value(path);
    }
  };

} // namespace std

namespace Hulks::GridCropper
{

  // NOLINTNEXTLINE(fuchsia-statically-constructed-objects)
  std::filesystem::path Processor::annotationsFileDirectory__;
  // NOLINTNEXTLINE(fuchsia-statically-constructed-objects)
  std::unordered_map<Processor::ItemType, std::vector<Circle<float>>> Processor::annotations__;

  std::vector<Processor::ItemType> Processor::prologue(Hulks::Runner::Runner<Processor>& runner,
                                                       const Configuration& configuration)
  {
    auto annotationsFile = configuration.outputAnnotationsFile;
    if (!annotationsFile.is_absolute())
    {
      annotationsFile = std::filesystem::absolute(annotationsFile);
    }
    annotationsFileDirectory__ = annotationsFile.parent_path();

    std::vector<ItemType> items;

    Hulks::Runner::Logger{runner} << "Collecting PNGs...";

    for (const auto& dataDirectoryOrFile : configuration.dataDirectoriesOrFiles)
    {
      const auto absoluteDataDirectoryOrFile = std::filesystem::absolute(dataDirectoryOrFile);
      std::error_code errorCode;
      if (std::filesystem::is_regular_file(absoluteDataDirectoryOrFile, errorCode))
      {
        if (absoluteDataDirectoryOrFile.extension() == ".png")
        {
          items.emplace_back(
              absoluteDataDirectoryOrFile.lexically_proximate(annotationsFileDirectory__));
        }
      }
      else if (!errorCode && std::filesystem::is_directory(absoluteDataDirectoryOrFile, errorCode))
      {
        for (const auto& path :
             std::filesystem::recursive_directory_iterator{absoluteDataDirectoryOrFile, errorCode})
        {
          if (path.is_regular_file() && path.path().extension() == ".png")
          {
            items.emplace_back(path.path().lexically_proximate(annotationsFileDirectory__));
          }
        }

        if (errorCode)
        {
          Hulks::Runner::Logger{runner} << "Directory " << absoluteDataDirectoryOrFile
                                        << ": iteration failed: " << errorCode.message();
          return {};
        }
      }
      else if (errorCode)
      {
        Hulks::Runner::Logger{runner} << "File " << absoluteDataDirectoryOrFile
                                      << ": stat retrieval failed: " << errorCode.message();
        return {};
      }
    }

    Hulks::Runner::Logger{runner} << "Collected " << items.size() << " PNGs.";

    std::sort(items.begin(), items.end());

    // reserve and rehash annotations map
    annotations__.reserve(items.size());

    return items;
  }

  void Processor::epilogue(Hulks::Runner::Runner<Processor>& runner,
                           const Configuration& configuration)
  {
    Hulks::Runner::Logger{runner} << "Writing annotations...";

    nlohmann::json annotations = nlohmann::json::object();
    std::size_t amountOfAnnotations{0};
    for (const auto& [imagePath, circles] : annotations__)
    {
      auto annotationCircles = nlohmann::json::array();
      for (const auto& circle : circles)
      {
        annotationCircles.emplace_back(nlohmann::json{
            {"centerX", circle.centerX}, {"centerY", circle.centerY}, {"radius", circle.radius}});
        ++amountOfAnnotations;
      }
      annotations[imagePath.string()] = annotationCircles;
    }

    std::ofstream jsonFile{configuration.outputAnnotationsFile,
                           std::ios_base::out | std::ios_base::trunc};
    if (!jsonFile.is_open())
    {
      Hulks::Runner::Logger{runner} << "File " << configuration.outputAnnotationsFile
                                    << " could not be opened";
      return;
    }

    jsonFile << annotations.dump(4) << '\n';

    Hulks::Runner::Logger{runner} << "Wrote " << amountOfAnnotations << " annotations of "
                                  << annotations__.size() << " input images.";
  }

  Processor::Processor(Hulks::Runner::Runner<Processor>& runner, const Configuration& configuration)
    : runner_{runner}
    , configuration_{configuration}
  {
    NeuralNetwork::Model classifierModel(configuration_.classifierModelPath);
    NeuralNetwork::Model positionerModel(configuration_.positionerModelPath);
    classifierCompiler_.compile(classifierModel);
    positionerCompiler_.compile(positionerModel);
  }

  void Processor::process(const Processor::ItemType& imagePath)
  {
    image_.data.reset(stbi_load((annotationsFileDirectory__ / imagePath).c_str(), &image_.width,
                                &image_.height, &image_.colorsPerPixel, 0));
    if (!image_.data)
    {
      Hulks::Runner::Logger{runner_} << "Failed to read " << imagePath;
      return;
    }

    generateCandidates();
    clusterCandidates();

    Hulks::Runner::Logger{runner_}
        << std::right << std::setw(6) << amountOfCandidates_ << " candidates, " << std::right
        << std::setw(6) << acceptedCandidates_.size() << " accepted, " << std::right << std::setw(6)
        << clusteredAcceptedCandidates_.size() << " clustered from " << imagePath;

    std::transform(clusteredAcceptedCandidates_.begin(), clusteredAcceptedCandidates_.end(),
                   std::back_inserter(annotations__[imagePath]),
                   [this](const Cluster& candidate) { return candidate.mergedCircle; });
  }

  void Processor::generateCandidates()
  {
    amountOfCandidates_ = 0;
    acceptedCandidates_.clear();

    // TODO: use non-linear step?
    // NOLINTNEXTLINE(clang-analyzer-security.FloatLoopCounter)
    for (float squareSize = static_cast<float>(image_.height) *
                            static_cast<float>(configuration_.maximumSquareSizeFactor);
         squareSize > static_cast<float>(configuration_.minimumSquareSize);
         // NOLINTNEXTLINE(cert-flp30-c)
         squareSize -= static_cast<float>(configuration_.squareSizeStep))
    {
      // NOLINTNEXTLINE(clang-analyzer-security.FloatLoopCounter)
      for (float centerY = 0.f; centerY < static_cast<float>(image_.height) + (squareSize / 2.f);
           // NOLINTNEXTLINE(cert-flp30-c)
           centerY += (squareSize / 2.f))
      {
        // NOLINTNEXTLINE(clang-analyzer-security.FloatLoopCounter)
        for (float centerX = 0.f; centerX < static_cast<float>(image_.width) + (squareSize / 2.f);
             // NOLINTNEXTLINE(cert-flp30-c)
             centerX += (squareSize / 2.f))
        {
          ++amountOfCandidates_;
          Circle<float> candidate{centerX, centerY, squareSize / 2.f};
          EvaluationResult evaluationResult{evaluateCandidate(candidate)};

          if (evaluationResult.isPositive)
          {
            // correct candidates position with the evaluationResult
            const float scale =
                static_cast<float>(squareSize) / static_cast<float>(configuration_.sampleSize);
            acceptedCandidates_.emplace_back(
                candidate,
                Circle<float>{(centerX - (squareSize / 2.f)) + evaluationResult.positionX * scale,
                              (centerY - (squareSize / 2.f)) + evaluationResult.positionY * scale,
                              evaluationResult.radius * scale},
                evaluationResult);
          }
        }
      }
    }
  }

  EvaluationResult Processor::evaluateCandidate(const Circle<float>& candidate)
  {
    EvaluationResult evaluationResult;

    if (!copySampleToTensor(candidate, classifierCompiler_.input(0)))
    {
      Hulks::Runner::Logger{runner_} << "Failed to resize sample";
      return evaluationResult;
    }

    // copy required before apply (else data is broken)
    positionerCompiler_.input(0) = classifierCompiler_.input(0);

    classifierCompiler_.apply();
    evaluationResult.ballConfidence = classifierCompiler_.output(0)[0];

    // skip positioner if classifier returned negative
    if (evaluationResult.ballConfidence > configuration_.ballConfidenceThreshold)
    {
      positionerCompiler_.apply();

      evaluationResult.isPositive = true;
      evaluationResult.positionX =
          positionerCompiler_.output(0)[0] * static_cast<float>(configuration_.sampleSize);
      evaluationResult.positionY =
          positionerCompiler_.output(0)[1] * static_cast<float>(configuration_.sampleSize);
      evaluationResult.radius =
          positionerCompiler_.output(0)[2] * static_cast<float>(configuration_.sampleSize) / 2.f;
    }

    return evaluationResult;
  }

  bool Processor::copySampleToTensor(const Circle<float>& candidate,
                                     NeuralNetwork::TensorXf& tensor) const
  {
    if ((configuration_.colorSpace == Configuration::ColorSpace::YCBCR ||
         configuration_.colorSpace == Configuration::ColorSpace::RGB) &&
        image_.colorsPerPixel != 3)
    {
      Hulks::Runner::Logger{runner_} << "Expected 3 colors per pixel but got "
                                     << image_.colorsPerPixel;
      return false;
    }
    if (configuration_.colorSpace == Configuration::ColorSpace::GRAYSCALE &&
        image_.colorsPerPixel != 1)
    {
      Hulks::Runner::Logger{runner_} << "Expected 1 colors per pixel but got "
                                     << image_.colorsPerPixel;
      return false;
    }

    // copy sampled candidate patch into tensor
    const auto upperLeftX = candidate.centerX - candidate.radius;
    const auto upperLeftY = candidate.centerY - candidate.radius;
    for (int y = 0; y < configuration_.sampleSize; ++y)
    {
      for (int x = 0; x < configuration_.sampleSize; ++x)
      {
        const auto pixelX = static_cast<int>(
            upperLeftX + (static_cast<float>(x) / static_cast<float>(configuration_.sampleSize) *
                          2.f * static_cast<float>(candidate.radius)));
        const auto pixelY = static_cast<int>(
            upperLeftY + (static_cast<float>(y) / static_cast<float>(configuration_.sampleSize) *
                          2.f * static_cast<float>(candidate.radius)));

        if (pixelX < 0 || pixelX >= image_.width || pixelY < 0 || pixelY >= image_.height)
        {
          tensor[y * configuration_.sampleSize + x] = configuration_.defaultColor;
          continue;
        }

        switch (configuration_.colorSpace)
        {
          case Configuration::ColorSpace::YCBCR:
          case Configuration::ColorSpace::GRAYSCALE:
            tensor[y * configuration_.sampleSize + x] =
                static_cast<float>(image_.data.get()[pixelY * image_.width * image_.colorsPerPixel +
                                                     pixelX * image_.colorsPerPixel]);
            break;
          case Configuration::ColorSpace::RGB:
            // Converting RGB to Y component via
            // https://en.wikipedia.org/wiki/YCbCr#JPEG_conversion
            tensor[y * configuration_.sampleSize + x] =
                0.299f * static_cast<float>(
                             image_.data.get()[pixelY * image_.width * image_.colorsPerPixel +
                                               pixelX * image_.colorsPerPixel]) +
                0.587f * static_cast<float>(
                             image_.data.get()[pixelY * image_.width * image_.colorsPerPixel +
                                               pixelX * image_.colorsPerPixel + 1]) +
                0.114f * static_cast<float>(
                             image_.data.get()[pixelY * image_.width * image_.colorsPerPixel +
                                               pixelX * image_.colorsPerPixel + 2]);
            break;
        }
      }
    }

    return true;
  }

  // https://math.stackexchange.com/a/99576
  float Processor::intersectionRatio(const float sampleXLeft, const float correctedXLeft,
                                     const float sampleXRight, const float correctedXRight,
                                     const float sampleYTop, const float correctedYTop,
                                     const float sampleYBottom, const float correctedYBottom)
  {
    const auto intersectionX = std::max(0.f, std::min(sampleXRight, correctedXRight) -
                                                 std::max(sampleXLeft, correctedXLeft));
    const auto intersectionY = std::max(0.f, std::min(sampleYBottom, correctedYBottom) -
                                                 std::max(sampleYTop, correctedYTop));

    const auto intersectionArea = intersectionX * intersectionY;
    const auto sampleArea = (sampleXRight - sampleXLeft) * (sampleYBottom - sampleYTop);

    return intersectionArea / sampleArea;
  }

  float Processor::circleIntersectionRatio(const Circle<float>& correctedCircle,
                                           const Circle<float>& sampleCircle)
  {
    return intersectionRatio(sampleCircle.centerX - sampleCircle.radius,
                             correctedCircle.centerX - correctedCircle.radius,
                             sampleCircle.centerX + sampleCircle.radius,
                             correctedCircle.centerX + correctedCircle.radius,

                             sampleCircle.centerY - sampleCircle.radius,
                             correctedCircle.centerY - correctedCircle.radius,
                             sampleCircle.centerY + sampleCircle.radius,
                             correctedCircle.centerY + correctedCircle.radius);
  }

  float Processor::imageIntersectionRatio(const Circle<float>& sampleCircle, const Image& image)
  {
    return intersectionRatio(
        sampleCircle.centerX - sampleCircle.radius, 0, sampleCircle.centerX + sampleCircle.radius,
        static_cast<float>(image.width),

        sampleCircle.centerY - sampleCircle.radius, 0, sampleCircle.centerY + sampleCircle.radius,
        static_cast<float>(image.height));
  }

  void Processor::clusterCandidates()
  {
    // The following code clusters the accepted candidates by distance.
    // It iterates over the accepted candidates (positively classified by the neural network),
    // for each accepted candidate: iterate over current existing/generated clusters and add
    // it to cluster if distance is below merge radius threshold. If there is no matching
    // cluster, a new cluster is added. For each added candidate, the merged circle of its
    // cluster is recalculated (also weighted by their confidence assigned by the neural
    // network).

    // contains clustered accepted candidates
    clusteredAcceptedCandidates_.clear();

    for (const auto& acceptedCandidate : acceptedCandidates_)
    {
      bool merged = false;
      // iterate over current clusters
      for (auto& clusteredAcceptedCandidate : clusteredAcceptedCandidates_)
      {
        // alias for the merged circle of the current cluster
        auto& mergedCircle{clusteredAcceptedCandidate.mergedCircle};
        // all candidates in the current cluster
        auto& candidatesInCluster{clusteredAcceptedCandidate.candidatesInCluster};
        // calculate distance between merged circle and new accepted candidate, if below merge
        // radius threshold:
        const float distance{
            (mergedCircle.centerX - acceptedCandidate.correctedCircle.centerX) *
                (mergedCircle.centerX - acceptedCandidate.correctedCircle.centerX) +
            (mergedCircle.centerY - acceptedCandidate.correctedCircle.centerY) *
                (mergedCircle.centerY - acceptedCandidate.correctedCircle.centerY)};
        if (distance < mergedCircle.radius * configuration_.mergeRadiusFactor *
                           mergedCircle.radius * configuration_.mergeRadiusFactor)
        {
          // add to existing cluster
          candidatesInCluster.emplace_back(acceptedCandidate);
          // recalculate merged circle
          mergedCircle.centerX = 0;
          mergedCircle.centerY = 0;
          mergedCircle.radius = 0;
          // while iterating over all candidates each candidate's circle gets added to the
          // merged circle (multiplied by a weight to ensure correct position) the weight of
          // each candidate in the merged circle is calculated by:
          //   candidateWeight = factor0^factor0Weight * factor1^factor1Weight * ...
          // to correctly scale the candidateWeight, the sum over all candidateWeights is
          // calculated
          const float sumOfCandidateWeights = std::accumulate(
              candidatesInCluster.begin(), candidatesInCluster.end(), 0.f,
              [this](auto accumulated, const auto& candidateInCluster) {
                const auto confidenceFactor = candidateInCluster.evaluationResult.ballConfidence;
                const auto correctionProximityFactor = circleIntersectionRatio(
                    candidateInCluster.correctedCircle, candidateInCluster.sampleCircle);
                const auto imageContainmentFactor =
                    imageIntersectionRatio(candidateInCluster.sampleCircle, image_);
                return accumulated +
                       std::pow(confidenceFactor, configuration_.confidenceFactorWeight) *
                           std::pow(correctionProximityFactor,
                                    configuration_.correctionProximityFactorWeight) *
                           std::pow(imageContainmentFactor,
                                    configuration_.imageContainmentFactorWeight);
              });

          // actually calculate new merged circle (considering weights)
          for (const auto& candidateInCluster : candidatesInCluster)
          {
            const auto confidenceFactor = candidateInCluster.evaluationResult.ballConfidence;
            const auto correctionProximityFactor = circleIntersectionRatio(
                candidateInCluster.correctedCircle, candidateInCluster.sampleCircle);
            const auto imageContainmentFactor =
                imageIntersectionRatio(candidateInCluster.sampleCircle, image_);
            const auto candidateWeight =
                (sumOfCandidateWeights == 0.f)
                    ? 1
                    : std::pow(confidenceFactor, configuration_.confidenceFactorWeight) *
                          std::pow(correctionProximityFactor,
                                   configuration_.correctionProximityFactorWeight) *
                          std::pow(imageContainmentFactor,
                                   configuration_.imageContainmentFactorWeight);
            mergedCircle.centerX += candidateInCluster.correctedCircle.centerX * candidateWeight;
            mergedCircle.centerY += candidateInCluster.correctedCircle.centerY * candidateWeight;
            mergedCircle.radius += candidateInCluster.correctedCircle.radius * candidateWeight;
          }

          if (sumOfCandidateWeights == 0.f)
          {
            mergedCircle.centerX /= static_cast<float>(candidatesInCluster.size());
            mergedCircle.centerY /= static_cast<float>(candidatesInCluster.size());
            mergedCircle.radius /= static_cast<float>(candidatesInCluster.size());
          }
          else
          {
            mergedCircle.centerX /= sumOfCandidateWeights;
            mergedCircle.centerY /= sumOfCandidateWeights;
            mergedCircle.radius /= sumOfCandidateWeights;
          }

          // advance to next accepted candidate
          merged = true;
          break;
        }
      }
      if (merged)
      {
        continue;
      }

      // append new cluster with accepted candidate
      clusteredAcceptedCandidates_.emplace_back(acceptedCandidate.correctedCircle,
                                                std::vector<CircleWithMetadata>{acceptedCandidate});
    }
  }

} // namespace Hulks::GridCropper
