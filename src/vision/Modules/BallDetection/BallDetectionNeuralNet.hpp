#pragma once

#include <Data/BallState.hpp>
#include <Data/CycleInfo.hpp>
#include <array>
#include <opencv2/dnn.hpp>
#include <opencv2/imgproc.hpp>
#include <vector>

#include "Data/BallSeeds.hpp"
#include "Data/BallData.hpp"
#include "Data/BoxCandidates.hpp"
#include "Data/CameraMatrix.hpp"
#include "Data/FieldBorder.hpp"
#include "Data/FieldColor.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/ImageData.hpp"
#include "Data/ImageSegments.hpp"
#include "Framework/Module.hpp"
#include "Tools/Math/Circle.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Storage/ObjectCandidate.hpp"

class Brain;

class BallDetectionNeuralNet : public Module<BallDetectionNeuralNet, Brain>
{
public:
  /// the name of this module
  ModuleName name = "BallDetectionNeuralNet";
  /**
   * @brief BallDetectionNeuralNet initializes members
   * @param manager a reference to brain
   */
  BallDetectionNeuralNet(const ModuleManagerInterface& manager);

  /**
   * @brief cycle tries to find a ball
   */
  void cycle() override;

private:
  const Dependency<BoxCandidates> boxCandidates_;
  const Dependency<BallSeeds> ballSeeds_;
  const Dependency<CameraMatrix> cameraMatrix_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<ImageData> imageData_;

  const Parameter<float> mergeRadiusFactor_;
  const Parameter<unsigned int> minSeedsInsideCandidateTop_;
  const Parameter<unsigned int> minSeedsInsideCandidateBottom_;
  const Parameter<std::string> networkPath_;
  const Parameter<float> softMaxThreshold_;
  const Parameter<bool> writeCandidatesToDisk_;
  const Parameter<bool> drawBallSeeds_;
  const Parameter<bool> drawDebugBoxes_;

  // the openCV network
  cv::dnn::Net network_;

  /// circles that should be drawn into the debug image
  std::vector<DebugCandidate<Circle<int>>> debugCandidates_;
  /// counter for candidate images if they are written to disk
  unsigned int candidateCount_;

  /**
   * @brief loads the frozen neural network specified in networkPath_
   */
  void loadNeuralNetwork();
  /*
   * @brief evaluates candidates whether they are balls
   * This method checks whether the given candidate is a new ball concerning pixel position. Then
   * the classifying CNN is inferred and the candidate is classified as ball or background. Balls
   * will be added to ballCandidates vector
   * @param candidate the candidate to evaluate
   * @param ballCandidates a reference to the vector of balls to store the classified ball
   */
  void evaluateCandidate(const ObjectCandidate& candidate,
                         std::vector<Circle<int>>& ballCandidates);
  /**
   * @brief takes a sample image and evaluates the network results
   * @param sample The sample image of the ball
   * @return -100.0 if the the sample is not of category ball or the output result
   */
  float infer(const std::vector<std::uint8_t>& sample);
  /**
   * @brief sends the debug image showing candidates and accepted/rejected balls
   */
  void sendDebugImage() const;
  /**
   * @brief writes all sample images evaluated by the neural net to disk
   */
  void writeCandidatesToDisk();

  /// the generated ball
  Production<BallData> ballData_;
};
