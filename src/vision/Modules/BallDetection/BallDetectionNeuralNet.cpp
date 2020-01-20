#include <cmath>
#include <fstream>
#include <iomanip>
#include <map>
#include <print.hpp>

#include "BallDetectionNeuralNet.hpp"
#include "Modules/Debug/Debug.h"
#include "Modules/Debug/PngConverter.h"
#include "Tools/Chronometer.hpp"

#include "BallDetectionNeuralNet.hpp"

BallDetectionNeuralNet::BallDetectionNeuralNet(const ModuleManagerInterface& manager)
  : Module(manager)
  , boxCandidates_(*this)
  , ballSeeds_(*this)
  , cameraMatrix_(*this)
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , imageData_(*this)
  , mergeRadiusFactor_(*this, "mergeRadiusFactor", [] {})
  , minSeedsInsideCandidateTop_(*this, "minSeedsInsideCandidateTop", [] {})
  , minSeedsInsideCandidateBottom_(*this, "minSeedsInsideCandidateBottom", [] {})
  , networkPath_(*this, "networkPath", [this] { this->loadNeuralNetwork(); })
  , softMaxThreshold_(*this, "softMaxThreshold", [] {})
  , writeCandidatesToDisk_(*this, "writeCandidatesToDisk", [] {})
  , drawBallSeeds_(*this, "drawDebugSeeds", [] {})
  , drawDebugBoxes_(*this, "drawDebugBoxes", [] {})
  , candidateCount_(0)
  , ballData_(*this)
{
  loadNeuralNetwork();
}

void BallDetectionNeuralNet::loadNeuralNetwork()
{
  const std::string networkPath = robotInterface().getDataRoot().append(networkPath_());
  network_ = cv::dnn::readNetFromTensorflow(networkPath);
}

void BallDetectionNeuralNet::evaluateCandidate(const ObjectCandidate& candidate,
                                               std::vector<Circle<int>>& acceptedCandidates)
{
  for (const auto& ball : acceptedCandidates)
  {
    if ((ball.center - candidate.center).norm() < ball.radius * mergeRadiusFactor_())
    {
      // candidate is near to already accepted ball candidate
      return;
    }
  }
  unsigned int seedsInsideCandidate = 0;
  for (const auto& seed : ballSeeds_->seeds)
  {
    if ((candidate.center - seed.position).squaredNorm() < candidate.radius * candidate.radius)
    {
      seedsInsideCandidate++;
    }
  }
  const unsigned int minSeedsInsideCandidate = imageData_->camera == Camera::TOP
                                                   ? minSeedsInsideCandidateTop_()
                                                   : minSeedsInsideCandidateBottom_();
  if (seedsInsideCandidate < minSeedsInsideCandidate)
  {
    debugCandidates_.emplace_back(candidate, Color::PINK);
    return;
  }
  // inference the CNN, drop if it's not in the ball class
  float cnnResult = infer(candidate.sample);

  if (cnnResult > softMaxThreshold_())
  {
    debugCandidates_.emplace_back(candidate, Color::GREEN);
    acceptedCandidates.push_back(candidate);
    return;
  }
  if (cnnResult > 0.5f)
  {
    debugCandidates_.emplace_back(candidate, Color::ORANGE);
    return;
  }
  debugCandidates_.emplace_back(candidate, Color::WHITE);
}

void BallDetectionNeuralNet::cycle()
{
  {
    Chronometer time(debug(), mount_ + ".cycle_time." + imageData_->identification);
    debugCandidates_.clear();

    std::vector<Circle<int>> acceptedCandidates;

    // loop over all candidates and check wheter they are accepted as ball by neural network
    for (const auto& candidate : boxCandidates_->candidates)
    {
      evaluateCandidate(candidate, acceptedCandidates);
    }
    // add all ball candidates accepted as balls to the ballData
    for (const auto& ball : acceptedCandidates)
    {
      Vector2f pos;
      cameraMatrix_->pixelToRobotWithZ(ball.center, fieldDimensions_->ballDiameter / 2, pos);
      ballData_->positions.push_back(pos);
      ballData_->imagePositions.push_back(ball);
    }
    ballData_->timestamp = imageData_->timestamp;
    ballData_->valid = true;
  }

  // send the debug image
  sendDebugImage();
  // if requested, write sampled candidates to disk
  if (writeCandidatesToDisk_() && gameControllerState_->gameState == GameState::PLAYING &&
      gameControllerState_->penalty == Penalty::NONE)
  {
    writeCandidatesToDisk();
  }
}

float BallDetectionNeuralNet::infer(const std::vector<std::uint8_t>& sample)
{
  const Chronometer time(debug(), mount_ + ".cycle_time.net");
  const int sampleSize = 15;
  // create CV matrix of size 15x15 and unsigned 8 bit 3 channel
  cv::Mat image(sampleSize, sampleSize, CV_8UC3);
  std::memcpy(image.data, sample.data(), sample.size() * sizeof(std::uint8_t));
  image.convertTo(image, CV_32FC3, 1.f / 255.f);

  // Create a 4D blob from a frame
  cv::Mat blob = cv::dnn::blobFromImage(image, 1.0, cv::Size(), cv::Scalar(), false);

  // Set input blob
  network_.setInput(blob);
  // Make forward pass
  cv::Mat outputs = network_.forward();

  return outputs.at<float>(1);
}

void BallDetectionNeuralNet::sendDebugImage() const
{
  const std::string debugImageMount = mount_ + "." + imageData_->identification + "_image";

  if (debug().isSubscribed(debugImageMount))
  {
    Image debugImage(imageData_->image422.to444Image());

    if (drawBallSeeds_())
    {
      for (const auto& seed422 : ballSeeds_->seeds)
      {
        const Circle<int> seed(Image422::get444From422Vector(seed422.position), seed422.radius);
        const int radiusHalf = seed.radius * 10 / 25;
        debugImage.line(Vector2i(seed.center.x() - radiusHalf, seed.center.y()),
                        Vector2i(seed.center.x() + radiusHalf, seed.center.y()), Color::BLUE);
        debugImage.line(Vector2i(seed.center.x(), seed.center.y() - radiusHalf),
                        Vector2i(seed.center.x(), seed.center.y() + radiusHalf), Color::BLUE);
        debugImage.line(Vector2i(seed.center.x() - radiusHalf, seed.center.y() - radiusHalf),
                        Vector2i(seed.center.x() + radiusHalf, seed.center.y() + radiusHalf),
                        Color::BLUE);
        debugImage.line(Vector2i(seed.center.x() + radiusHalf, seed.center.y() - radiusHalf),
                        Vector2i(seed.center.x() - radiusHalf, seed.center.y() + radiusHalf),
                        Color::BLUE);
      }
    }
    if (drawDebugBoxes_())
    {
      for (const auto& debugBox : boxCandidates_->debugBoxes)
      {
        Rectangle<int> box(debugBox.candidate.center - Vector2i::Ones() * debugBox.candidate.radius,
                           debugBox.candidate.center +
                               Vector2i::Ones() * debugBox.candidate.radius);
        box.from422to444();
        debugImage.rectangle(box, debugBox.color);
      }
    }
    for (auto& debugCircle : debugCandidates_)
    {
      Circle<int> circle(debugCircle.candidate);
      circle.from422to444();
      debugImage.cross(circle.center, 3, debugCircle.color);
      debugImage.circle(circle.center, circle.radius - 1, debugCircle.color);
      debugImage.circle(circle.center, circle.radius, debugCircle.color);
      debugImage.circle(circle.center, circle.radius + 1, debugCircle.color);
    }
    debug().sendImage(debugImageMount, debugImage);
  }
}

void BallDetectionNeuralNet::writeCandidatesToDisk()
{
  PngConverter pngConverter;
  for (const auto& candidate : debugCandidates_)
  {
    const Vector2i from(candidate.candidate.center.x() - candidate.candidate.radius / 2,
                        candidate.candidate.center.y() - candidate.candidate.radius);
    const Vector2i to(candidate.candidate.center.x() + candidate.candidate.radius / 2,
                      candidate.candidate.center.y() + candidate.candidate.radius);
    Image422 ballCandidateSample(to - from);
    for (int x = 0; x < to.x() - from.x(); x++)
    {
      for (int y = 0; y < to.y() - from.y(); y++)
      {
        Vector2i point(x, y);
        if (imageData_->image422.isInside(from + point))
        {
          ballCandidateSample[point] = imageData_->image422[from + point];
        }
        else
        {
          const uint8_t fallback = 128;
          ballCandidateSample[point] = YCbCr422(fallback, fallback, fallback, fallback);
        }
      }
    }
    std::ofstream fs;
    CVData image;
    pngConverter.convert(ballCandidateSample.to444Image(), image);
    std::string fn = mount_;
    if (candidate.color == Color::GREEN)
    {
      fn += ".true";
    }
    else if (candidate.color == Color::WHITE)
    {
      fn += ".false";
    }
    else
    {
      continue;
    }
    fn = robotInterface().getFileRoot() + "filetransport_ball_candidates/" + fn + "." +
         std::to_string(candidateCount_++) + ".png";
    fs.open(fn, std::ios_base::out | std::ios_base::trunc | std::ios_base::binary);
    fs.write(reinterpret_cast<const char*>(image.data()), image.size());
    fs.close();
  }
}
