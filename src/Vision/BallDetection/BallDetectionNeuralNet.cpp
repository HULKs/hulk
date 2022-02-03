#include "Vision/BallDetection/BallDetectionNeuralNet.hpp"
#include "Framework/Debug/Debug.h"
#include "Framework/Debug/PngConverter.h"
#include "Tools/Chronometer.hpp"
#include <cmath>
#include <fstream>
#include <map>
#include <vector>

BallDetectionNeuralNet::BallDetectionNeuralNet(const ModuleManagerInterface& manager)
  : Module{manager}
  , perspectiveGridCandidates_{*this}
  , cameraMatrix_{*this}
  , fieldDimensions_{*this}
  , imageData_{*this}
  , mergeRadiusFactor_{*this, "mergeRadiusFactor", [] {}}
  , confidenceThresholdPreClassifier_{*this, "confidenceThresholdPreClassifier", [] {}}
  , confidenceThresholdPreClassifierDebug_{*this, "confidenceThresholdPreClassifierDebug", [] {}}
  , confidenceThresholdClassifier_{*this, "confidenceThresholdClassifier", [] {}}
  , confidenceThresholdClassifierDebug_{*this, "confidenceThresholdClassifierDebug", [] {}}
  , confidenceFactorWeight_{*this, "confidenceFactorWeight", [] {}}
  , correctionProximityFactorWeight_{*this, "correctionProximityFactorWeight", [] {}}
  , imageContainmentFactorWeight_{*this, "imageContainmentFactorWeight", [] {}}
  , recordAllPositives_{*this, "recordAllPositives", [] {}}
  , recordIfNumberOfPositivesIncreases_{*this, "recordIfNumberOfPositivesIncreases", [] {}}
  , recordIfNumberOfPositivesDecreases_{*this, "recordIfNumberOfPositivesDecreases", [] {}}
  , drawPreCandidateOutlines_{*this, "drawPreCandidateOutlines", [] {}}
  , drawPreCandidateAnnotations_{*this, "drawPreCandidateAnnotations", [] {}}
  , drawDebugCandidateOutlines_{*this, "drawDebugCandidateOutlines", [] {}}
  , drawDebugCandidateAnnotations_{*this, "drawDebugCandidateAnnotations", [] {}}
  , drawCandidateOutlines_{*this, "drawCandidateOutlines", [] {}}
  , drawCandidateAnnotations_{*this, "drawCandidateAnnotations", [] {}}
  , drawDebugBallOutlines_{*this, "drawDebugBallOutlines", [] {}}
  , drawDebugBallAnnotations_{*this, "drawDebugBallAnnotations", [] {}}
  , drawBallOutlines_{*this, "drawBallOutlines", [] {}}
  , drawBallAnnotations_{*this, "drawBallAnnotations", [] {}}
  , drawClusteringAnnotations_{*this, "drawClusteringAnnotations", [] {}}
  , sampleSize_{*this, "sampleSize"}
  , ballRadiusIncreaseFactor_{*this, "ballRadiusIncreaseFactor", [] {}}
  , preclassifierPath_{*this, "preclassifierPath", [this] { this->loadNeuralNetwork(); }}
  , classifierPath_{*this, "classifierPath", [this] { this->loadNeuralNetwork(); }}
  , positionerPath_{*this, "positionerPath", [this] { this->loadNeuralNetwork(); }}
  , preclassifierCompilationSettings_{*this, "preclassifierCompilationSettings",
                                      [this] { this->loadNeuralNetwork(); }}
  , classifierCompilationSettings_{*this, "classifierCompilationSettings",
                                   [this] { this->loadNeuralNetwork(); }}
  , positionerCompilationSettings_{*this, "positionerCompilationSettings",
                                   [this] { this->loadNeuralNetwork(); }}
  , numberOfLastPositivesTop_{0}
  , numberOfLastPositivesBottom_{0}
  , ballData_{*this}
  , ballDetectionReplayRecorderData_{*this}
{
  loadNeuralNetwork();
}

void BallDetectionNeuralNet::cycle()
{
  {
    Chronometer time(debug(), mount_ + ".cycle_time." + imageData_->identification);
    debugImageMount_ = mount_ + "." + imageData_->identification + "_image";
    if (!perspectiveGridCandidates_->valid || !cameraMatrix_->valid || !imageData_->valid)
    {
      return;
    }

    evaluateCandidates();

    clusterCandidates();

    updateReplayRecorderData();

    // populate BallData production based on clustered candidates
    for (const auto& cluster : clusters_)
    {
      const std::optional<Vector2f> pos = cameraMatrix_->pixelToRobotWithZ(
          cluster.mergedCircle.center.cast<int>(), fieldDimensions_->ballDiameter / 2.f);
      if (!pos.has_value())
      {
        continue;
      }
      ballData_->positions.emplace_back(pos.value());
      ballData_->imagePositions.emplace_back(cluster.mergedCircle.center.cast<int>(),
                                             static_cast<int>(cluster.mergedCircle.radius));
    }
    ballData_->timestamp = imageData_->captureTimePoint;
    ballData_->valid = true;
  }

  // send the debug image
  sendDebugImage();
}

void BallDetectionNeuralNet::loadNeuralNetwork()
{
  const std::lock_guard<std::mutex> lock(compilerMutex_);

  NeuralNetwork::Model preclassifierModel(
      robotInterface().getFileRoot().append(preclassifierPath_()));
  NeuralNetwork::Model classifierModel(robotInterface().getFileRoot().append(classifierPath_()));
  NeuralNetwork::Model positionerModel(robotInterface().getFileRoot().append(positionerPath_()));
  preclassifierCompiler_.compile(preclassifierModel,
                                 preclassifierCompilationSettings_().toCompilationSettings());
  classifierCompiler_.compile(classifierModel,
                              classifierCompilationSettings_().toCompilationSettings());
  positionerCompiler_.compile(positionerModel,
                              positionerCompilationSettings_().toCompilationSettings());
}

void BallDetectionNeuralNet::evaluateCandidates()
{
  candidates_.clear();

  // evaluate candidates
  for (const auto& candidateCircle : perspectiveGridCandidates_->candidates)
  {
    auto& candidate = candidates_.emplace_back();
    candidate.candidateCircle = candidateCircle;
    candidate.sizeInImage444 =
        2.f * static_cast<float>(candidateCircle.radius) * ballRadiusIncreaseFactor_();
    candidate.scale444 = candidate.sizeInImage444 / static_cast<float>(sampleSize_());

    const std::lock_guard<std::mutex> lock(compilerMutex_);
    evaluatePreClassifier(candidate);
    if (candidate.preClassifierConfidence < confidenceThresholdPreClassifier_())
    {
      continue;
    }

    evaluateClassifier(candidate);
    if (candidate.classifierConfidence < confidenceThresholdClassifier_())
    {
      continue;
    }

    evaluatePositioner(candidate);
  }
}

void BallDetectionNeuralNet::evaluatePreClassifier(CandidateMetadata& candidate)
{
  sampleBoundingBox(candidate, preclassifierCompiler_.input(0));
  preclassifierCompiler_.apply();
  candidate.preClassifierConfidence = preclassifierCompiler_.output(0)[0];
}

void BallDetectionNeuralNet::evaluateClassifier(CandidateMetadata& candidate)
{
  sampleBoundingBox(candidate, classifierCompiler_.input(0));

  // copy required before apply (else data is broken)
  positionerCompiler_.input(0) = classifierCompiler_.input(0);

  classifierCompiler_.apply();
  candidate.classifierConfidence = classifierCompiler_.output(0)[0];
}

void BallDetectionNeuralNet::evaluatePositioner(CandidateMetadata& candidate)
{
  positionerCompiler_.apply();
  candidate.positionX = positionerCompiler_.output(0)[0] * static_cast<float>(sampleSize_());
  candidate.positionY = positionerCompiler_.output(0)[1] * static_cast<float>(sampleSize_());
  candidate.radius = positionerCompiler_.output(0)[2] * static_cast<float>(sampleSize_()) / 2.f;

  // correct candidates position
  const Vector2f positionCorrection422 = {
      (candidate.positionX - static_cast<float>(sampleSize_()) / 2.f) * candidate.scale444 / 2.f,
      (candidate.positionY - static_cast<float>(sampleSize_()) / 2.f) * candidate.scale444};
  candidate.correctedCircle = {candidate.candidateCircle.center.cast<float>() +
                                   positionCorrection422,
                               static_cast<float>(candidate.radius) * candidate.scale444};
}

void BallDetectionNeuralNet::sampleBoundingBox(const CandidateMetadata& candidate,
                                               NeuralNetwork::TensorXf& sampledPatch) const
{
  const Vector2i from444{
      static_cast<int>(static_cast<float>(candidate.candidateCircle.center.x() * 2.f) -
                       candidate.sizeInImage444 / 2.f),
      static_cast<int>(static_cast<float>(candidate.candidateCircle.center.y()) -
                       candidate.sizeInImage444 / 2.f)};

  Vector2i pixel(from444);
  for (unsigned int y = 0; y < sampleSize_(); y++)
  {
    pixel.y() = from444.y() + static_cast<int>(y * candidate.scale444);
    for (unsigned int x = 0; x < sampleSize_(); x++)
    {
      // first, calculate x position in YUV444 coords
      pixel.x() = from444.x() + static_cast<int>(x * candidate.scale444);
      // check if 444 coord is even
      const bool xEven = pixel.x() % 2 == 0;
      // convert to 422 coordinate
      pixel.x() /= 2;
      // fallback to 128 if pixel is not inside image
      if (!imageData_->image422.isInside(pixel))
      {
        constexpr std::uint8_t fallback = 128;
        sampledPatch[y * sampleSize_() + x] = static_cast<float>(fallback);
        continue;
      }
      // get 422 color
      const YCbCr422& color = imageData_->image422[pixel];
      // if 444 coord was even, take the first y value, otherwise the second
      const std::uint8_t& yByte = xEven ? color.y1 : color.y2;
      sampledPatch[y * sampleSize_() + x] = static_cast<float>(yByte);
    }
  }
}

void BallDetectionNeuralNet::updateReplayRecorderData()
{
  // calculate positives from last cycle
  const auto& lastCandidates = imageData_->cameraPosition == CameraPosition::TOP
                                   ? lastCandidatesTop_
                                   : lastCandidatesBottom_;
  const auto numberOfLastPositives = imageData_->cameraPosition == CameraPosition::TOP
                                         ? numberOfLastPositivesTop_
                                         : numberOfLastPositivesBottom_;

  // generate ball detection replay data and count positives in current cycle
  std::vector<BallDetectionData::CandidateCircle> replayCandidates;
  int numberOfCurrentPositives = 0;
  for (const auto& candidate : candidates_)
  {
    const auto circle = candidate.candidateCircle.get444from422();
    replayCandidates.emplace_back(candidate.preClassifierConfidence, candidate.classifierConfidence,
                                  circle);
    if (candidate.preClassifierConfidence >= confidenceThresholdPreClassifier_() &&
        candidate.classifierConfidence >= confidenceThresholdClassifier_())
    {
      ++numberOfCurrentPositives;
    }
  }

  // request recording of this cycle if it is requested by the config
  // this records when a positive candidate has been seen
  // OR
  // request recording of this cycle if it is requested by the config
  // this records when the number of detected balls changes from the last cycle
  const bool requestRecordAllPositives = recordAllPositives_() && numberOfCurrentPositives > 0;
  const bool requestRecordIfNumberOfPositivesIncreases =
      recordIfNumberOfPositivesIncreases_() && numberOfCurrentPositives > numberOfLastPositives;
  const bool requestRecordIfNumberOfPositivesDecreases =
      recordIfNumberOfPositivesDecreases_() && numberOfCurrentPositives < numberOfLastPositives;
  if (requestRecordAllPositives || requestRecordIfNumberOfPositivesIncreases ||
      requestRecordIfNumberOfPositivesDecreases)
  {
    ballDetectionReplayRecorderData_->recordingRequested = true;
    ballDetectionReplayRecorderData_->data.candidates = replayCandidates;
    ballDetectionReplayRecorderData_->data.lastCandidates = lastCandidates;

    // also send clusters
    std::transform(clusters_.begin(), clusters_.end(),
                   std::back_inserter(ballDetectionReplayRecorderData_->data.clusters),
                   [](const auto& cluster) {
                     std::vector<BallDetectionData::Cluster::Candidate> candidatesInCluster;
                     std::transform(
                         cluster.candidatesInCluster.begin(), cluster.candidatesInCluster.end(),
                         std::back_inserter(candidatesInCluster), [](const auto& candidate) {
                           return BallDetectionData::Cluster::Candidate{
                               candidate->candidateCircle.get444from422(),
                               candidate->correctedCircle.get444from422()};
                         });

                     return BallDetectionData::Cluster{cluster.mergedCircle.get444from422(),
                                                       candidatesInCluster};
                   });
  }

  // update last ball detection number stats
  if (imageData_->cameraPosition == CameraPosition::TOP)
  {
    lastCandidatesTop_ = replayCandidates;
    numberOfLastPositivesTop_ = numberOfCurrentPositives;
  }
  else if (imageData_->cameraPosition == CameraPosition::BOTTOM)
  {
    lastCandidatesBottom_ = replayCandidates;
    numberOfLastPositivesBottom_ = numberOfCurrentPositives;
  }
}

void BallDetectionNeuralNet::sendDebugImage() const
{
  if (debug().isSubscribed(debugImageMount_))
  {
    Image debugImage(imageData_->image422.to444Image());

    for (const auto& candidate : candidates_)
    {
      Color color{Color::BLUE};
      std::string annotation;
      bool outlineEnabled = false;
      bool annotationEnabled = false;

      if (candidate.preClassifierConfidence < confidenceThresholdPreClassifierDebug_())
      {
        color = Color::BLUE;
        annotation =
            std::to_string(static_cast<int>(std::round(candidate.preClassifierConfidence * 100))) +
            "%";
        outlineEnabled = drawPreCandidateOutlines_();
        annotationEnabled = drawPreCandidateAnnotations_();
      }
      else if (candidate.preClassifierConfidence < confidenceThresholdPreClassifier_())
      {
        color = Color::RED;
        annotation =
            std::to_string(static_cast<int>(std::round(candidate.preClassifierConfidence * 100))) +
            "%";
        outlineEnabled = drawDebugCandidateOutlines_();
        annotationEnabled = drawDebugCandidateAnnotations_();
      }
      else if (candidate.classifierConfidence < confidenceThresholdClassifierDebug_())
      {
        color = Color::ORANGE;
        annotation =
            std::to_string(static_cast<int>(std::round(candidate.preClassifierConfidence * 100))) +
            "%\n" +
            std::to_string(static_cast<int>(std::round(candidate.classifierConfidence * 100))) +
            "%";
        outlineEnabled = drawCandidateOutlines_();
        annotationEnabled = drawCandidateAnnotations_();
      }
      else if (candidate.classifierConfidence < confidenceThresholdClassifier_())
      {
        color = Color::YELLOW;
        annotation =
            std::to_string(static_cast<int>(std::round(candidate.preClassifierConfidence * 100))) +
            "%\n" +
            std::to_string(static_cast<int>(std::round(candidate.classifierConfidence * 100))) +
            "%";
        outlineEnabled = drawDebugBallOutlines_();
        annotationEnabled = drawDebugBallAnnotations_();
      }
      else if (candidate.classifierConfidence >= confidenceThresholdClassifier_())
      {
        color = Color::GREEN;
        annotation =
            std::to_string(static_cast<int>(std::round(candidate.preClassifierConfidence * 100))) +
            "%\n" +
            std::to_string(static_cast<int>(std::round(candidate.classifierConfidence * 100))) +
            "%";
        outlineEnabled = drawBallOutlines_();
        annotationEnabled = drawBallAnnotations_();
      }

      if (outlineEnabled)
      {
        const float radius = candidate.candidateCircle.radius * ballRadiusIncreaseFactor_();
        Rectangle<int> box({(candidate.candidateCircle.center.x() * 2) - radius,
                            candidate.candidateCircle.center.y() - radius},
                           {(candidate.candidateCircle.center.x() * 2) + radius,
                            candidate.candidateCircle.center.y() + radius});
        debugImage.drawRectangle(box, color);
      }
      if (annotationEnabled)
      {
        auto circle = candidate.candidateCircle.get444from422();
        circle.radius *= ballRadiusIncreaseFactor_();
        if (outlineEnabled)
        {
          debugImage.drawText(annotation,
                              circle.center.cast<int>() + Vector2i{static_cast<int>(-circle.radius),
                                                                   static_cast<int>(circle.radius)},
                              color);
        }
        else
        {
          debugImage.drawText(annotation, circle.center.cast<int>(), color);
        }
      }
    }

    if (drawBallOutlines_())
    {
      for (const auto& cluster : clusters_)
      {
        Circle<int> circle{cluster.mergedCircle.center.cast<int>(),
                           static_cast<int>(cluster.mergedCircle.radius)};
        circle.convertFrom422to444();
        debugImage.drawCross(circle.center, 3, Color::GREEN);
        debugImage.drawCircle(circle.center, circle.radius - 1, Color::GREEN);
        debugImage.drawCircle(circle.center, circle.radius, Color::GREEN);
        debugImage.drawCircle(circle.center, circle.radius + 1, Color::GREEN);
      }
    }

    if (drawClusteringAnnotations_())
    {
      debugImage.drawText(debugStringsOfClustering_, Vector2i::Zero(), Color::YELLOW);
    }

    debug().sendImage(debugImageMount_, debugImage);
  }
}

void BallDetectionNeuralNet::clusterCandidates()
{
  clusters_.clear();
  debugStringsOfClustering_.clear();

  // The following code clusters the accepted candidates by distance.
  // It iterates over the accepted candidates (positively classified by the neural network), for
  // each accepted candidate: iterate over current existing/generated clusters and add it to
  // cluster if distance is below merge radius threshold. If there is no matching cluster, a new
  // cluster is added. For each added candidate, the merged circle of its cluster is recalculated
  // (also weighted by their confidence and other metrics).
  for (auto candidatesIterator = candidates_.begin(); candidatesIterator != candidates_.end();
       ++candidatesIterator)
  {
    if (candidatesIterator->preClassifierConfidence < confidenceThresholdPreClassifier_() ||
        candidatesIterator->classifierConfidence < confidenceThresholdClassifier_())
    {
      continue;
    }

    bool merged = false;
    // iterate over current clusters
    for (auto clusteredCandidatesIterator = clusters_.begin();
         clusteredCandidatesIterator != clusters_.end(); ++clusteredCandidatesIterator)
    {
      // alias for the merged circle of the current cluster
      auto& mergedCircle = clusteredCandidatesIterator->mergedCircle;
      // all candidates in the current cluster
      auto& candidatesInCluster = clusteredCandidatesIterator->candidatesInCluster;
      // calculate distance between merged circle and new accepted candidate, if below merge
      // radius threshold:
      if ((mergedCircle.center - candidatesIterator->correctedCircle.center).squaredNorm() <
          mergedCircle.radius * mergeRadiusFactor_() * mergedCircle.radius * mergeRadiusFactor_())
      {
        // add to existing cluster
        candidatesInCluster.emplace_back(candidatesIterator);
        // recalculate merged circle
        mergedCircle.center = Vector2f::Zero();
        mergedCircle.radius = 0;
        // while iterating over all candidates each candidate's circle gets added to the merged
        // circle (multiplied by a weight to ensure correct position)
        // the weight of each candidate in the merged circle is calculated by:
        //   candidateWeight = factor0^factor0Weight * factor1^factor1Weight * ...
        // to correctly scale the candidateWeight, the sum over all candidateWeights is calculated
        const float sumOfCandidateWeights = std::accumulate(
            candidatesInCluster.begin(), candidatesInCluster.end(), 0.f,
            [this](auto accumulated, const auto& candidateInCluster) {
              const auto confidenceFactor = candidateInCluster->classifierConfidence;
              const auto correctionProximityFactor = circleIntersectionRatio(
                  candidateInCluster->correctedCircle,
                  {candidateInCluster->correctedCircle.center.template cast<float>(),
                   static_cast<float>(candidateInCluster->correctedCircle.radius)});
              const auto imageContainmentFactor = imageIntersectionRatio(
                  {candidateInCluster->candidateCircle.center.template cast<float>(),
                   static_cast<float>(candidateInCluster->candidateCircle.radius)},
                  imageData_->image422.size);
              return accumulated +
                     std::pow(confidenceFactor, confidenceFactorWeight_()) *
                         std::pow(correctionProximityFactor, correctionProximityFactorWeight_()) *
                         std::pow(imageContainmentFactor, imageContainmentFactorWeight_());
            });
        // actually calculate new merged circle (considering weights)
        for (const auto& candidateInCluster : candidatesInCluster)
        {
          const auto confidenceFactor = candidateInCluster->classifierConfidence;
          const auto correctionProximityFactor = circleIntersectionRatio(
              candidateInCluster->correctedCircle,
              {candidateInCluster->correctedCircle.center.cast<float>(),
               static_cast<float>(candidateInCluster->correctedCircle.radius)});
          const auto imageContainmentFactor = imageIntersectionRatio(
              {candidateInCluster->candidateCircle.center.cast<float>(),
               static_cast<float>(candidateInCluster->candidateCircle.radius)},
              imageData_->image422.size);
          const auto candidateWeight =
              (sumOfCandidateWeights == 0.f)
                  ? 1
                  : std::pow(confidenceFactor, confidenceFactorWeight_()) *
                        std::pow(correctionProximityFactor, correctionProximityFactorWeight_()) *
                        std::pow(imageContainmentFactor, imageContainmentFactorWeight_());
          mergedCircle.center += candidateInCluster->correctedCircle.center * candidateWeight;
          mergedCircle.radius += candidateInCluster->correctedCircle.radius * candidateWeight;
        }

        if (sumOfCandidateWeights == 0.f)
        {
          assert(candidatesInCluster.size() > 0);
          mergedCircle.center /= static_cast<float>(candidatesInCluster.size());
          mergedCircle.radius /= static_cast<float>(candidatesInCluster.size());
        }
        else
        {
          mergedCircle.center /= sumOfCandidateWeights;
          mergedCircle.radius /= sumOfCandidateWeights;
        }

        // save a string for drawing in the debug image
        if (drawClusteringAnnotations_() && debug().isSubscribed(debugImageMount_))
        {
          debugStringsOfClustering_ +=
              "Append to #" +
              std::to_string(std::distance(clusters_.begin(), clusteredCandidatesIterator)) +
              ": (x=" +
              std::to_string(static_cast<int>(candidatesIterator->correctedCircle.center.x())) +
              ", y=" +
              std::to_string(static_cast<int>(candidatesIterator->correctedCircle.center.y())) +
              ", r=" +
              std::to_string(static_cast<int>(candidatesIterator->correctedCircle.radius)) + ")\n";
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
    clusters_.emplace_back(
        candidatesIterator->correctedCircle,
        std::vector<std::vector<CandidateMetadata>::const_iterator>{candidatesIterator});

    // save a string for drawing in the debug image
    if (drawClusteringAnnotations_() && debug().isSubscribed(debugImageMount_))
    {
      debugStringsOfClustering_ +=
          "New #" + std::to_string(clusters_.size() - 1) + ": (x=" +
          std::to_string(static_cast<int>(candidatesIterator->correctedCircle.center.x())) +
          ", y=" +
          std::to_string(static_cast<int>(candidatesIterator->correctedCircle.center.y())) +
          ", r=" + std::to_string(static_cast<int>(candidatesIterator->correctedCircle.radius)) +
          ")\n";
    }
  }
}

float BallDetectionNeuralNet::intersectionRatio(float sampleXLeft, float correctedXLeft,
                                                float sampleXRight, float correctedXRight,
                                                float sampleYTop, float correctedYTop,
                                                float sampleYBottom, float correctedYBottom)
{
  // https://math.stackexchange.com/a/99576
  const auto intersectionX = std::max(0.f, std::min(sampleXRight, correctedXRight) -
                                               std::max(sampleXLeft, correctedXLeft));
  const auto intersectionY = std::max(0.f, std::min(sampleYBottom, correctedYBottom) -
                                               std::max(sampleYTop, correctedYTop));

  const auto intersectionArea = intersectionX * intersectionY;
  const auto sampleArea = (sampleXRight - sampleXLeft) * (sampleYBottom - sampleYTop);

  return intersectionArea / sampleArea;
}

float BallDetectionNeuralNet::circleIntersectionRatio(const Circle<float>& correctedCircle,
                                                      Circle<float> sampleCircle)
{
  return intersectionRatio(sampleCircle.center.x() - sampleCircle.radius,
                           correctedCircle.center.x() - correctedCircle.radius,
                           sampleCircle.center.x() + sampleCircle.radius,
                           correctedCircle.center.x() + correctedCircle.radius,

                           sampleCircle.center.y() - sampleCircle.radius,
                           correctedCircle.center.y() - correctedCircle.radius,
                           sampleCircle.center.y() + sampleCircle.radius,
                           correctedCircle.center.y() + correctedCircle.radius);
}

float BallDetectionNeuralNet::imageIntersectionRatio(Circle<float> sampleCircle,
                                                     const Vector2i& imageSize)
{
  return intersectionRatio(
      sampleCircle.center.x() - sampleCircle.radius, 0,
      sampleCircle.center.x() + sampleCircle.radius, static_cast<float>(imageSize.x()),

      sampleCircle.center.y() - sampleCircle.radius, 0,
      sampleCircle.center.y() + sampleCircle.radius, static_cast<float>(imageSize.y()));
}

BallDetectionNeuralNet::Cluster::Cluster(
    Circle<float> mergedCircle,
    std::vector<std::vector<CandidateMetadata>::const_iterator> candidatesInCluster)
  : mergedCircle{std::move(mergedCircle)}
  , candidatesInCluster{std::move(candidatesInCluster)}
{
}

NeuralNetwork::CompilationSettings
BallDetectionNeuralNet::CompilationSettings::toCompilationSettings() const
{
  NeuralNetwork::CompilationSettings settings;
  settings.useX64 = useX64;
  settings.useSSE42 = useSSE42;
  settings.useAVX2 = useAVX2;
  settings.useExpApproxInSigmoid = useExponentialApproximationInSigmoid;
  settings.useExpApproxInTanh = useExponentialApproximationInTanh;
  return settings;
}

void BallDetectionNeuralNet::CompilationSettings::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::OBJECT);
  value["useX64"] << useX64;
  value["useSSE42"] << useSSE42;
  value["useAVX2"] << useAVX2;
  value["useExponentialApproximationInSigmoid"] << useExponentialApproximationInSigmoid;
  value["useExponentialApproximationInTanh"] << useExponentialApproximationInTanh;
}

void BallDetectionNeuralNet::CompilationSettings::fromValue(const Uni::Value& value)
{
  value["useX64"] >> useX64;
  value["useSSE42"] >> useSSE42;
  value["useAVX2"] >> useAVX2;
  value["useExponentialApproximationInSigmoid"] >> useExponentialApproximationInSigmoid;
  value["useExponentialApproximationInTanh"] >> useExponentialApproximationInTanh;
}
