#pragma once

#include <Data/BallState.hpp>
#include <Data/CycleInfo.hpp>
#include <array>
#include <vector>

#include "Data/BallData.hpp"
#include "Data/CameraMatrix.hpp"
#include "Data/FieldBorder.hpp"
#include "Data/FieldColor.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/FilteredRegions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/ImageData.hpp"
#include "Framework/Module.hpp"
#include "Tools/Math/Circle.hpp"
#include "Tools/Math/Eigen.hpp"

class Brain;

class BallDetectionNeuralNet : public Module<BallDetectionNeuralNet, Brain>
{
public:
  /**
   * @brief BallDetectionNeuralNet initializes members
   * @param manager a reference to brain
   */
  BallDetectionNeuralNet(const ModuleManagerInterface& manager);

  /**
   * @brief cycle tries to find a ball
   */
  void cycle();

private:
  typedef unsigned int dim_t;

  template <typename T>
  T sqr(const T x) const
  {
    return x * x;
  }

  float scaleByte(const unsigned char foo) const
  {
    return foo / 128.f - 1.f;
  }

  struct MergedCandidate
  {
    Circle<int> candidate;
    unsigned int count;
  };

  /**
   * @brief DebugCircle is a combination of a circle and a color in which the circle should be drawn
   */
  struct DebugCircle
  {
    /**
     * @brief DebugCircle initializes a debug circle with given circle and color
     * @param circle a circle in image coordinates
     * @param color the color in which to draw the circle
     */
    DebugCircle(const Circle<int>& circle, const Color color)
      : circle(circle)
      , color(color)
    {
    }

    /// a circle in image coordinates
    Circle<int> circle;
    /// the color in which to draw the circle
    Color color;
  };

  /// generates ball candidates from image
  std::vector<Circle<int>> getSeeds();
  std::vector<Circle<int>> mergeSeeds(const std::vector<Circle<int>>& seeds);
  bool projectFoundBall(Circle<int>& foundBall) const;

  /// applies filterBy* - functions to every candidate, creating a list of balls
  bool applyFilter(const std::vector<Circle<int>>& candidates, std::vector<Circle<int>>& balls);

  /// functions for execution CNNs
  void activate(std::vector<float>& img, const int activation) const;
  void normalize(std::vector<float>& img, const std::vector<std::vector<float>>& norm) const;
  void pool(const std::vector<float>& img, const std::array<dim_t, 3>& inDim, const int poolType, std::vector<float>& result,
            std::array<dim_t, 3>& outDim) const;
  void maxPool2x2(const std::vector<float>& img, const std::array<dim_t, 3>& inDim, std::vector<float>& result, std::array<dim_t, 3>& outDim) const;
  void avgPool2x2(const std::vector<float>& img, const std::array<dim_t, 3>& inDim, std::vector<float>& result, std::array<dim_t, 3>& outDim) const;
  void execLayer(const std::vector<float>& input, const std::vector<std::vector<float>>& weights, const std::vector<float>& bias, const int activation,
                 std::vector<float>& output) const;
  void convolution(const std::vector<float>& input, const std::array<dim_t, 3>& inDim, const std::vector<std::vector<std::vector<std::vector<float>>>>& mask,
                   const std::vector<float>& bias, const int activation, const int pooling, std::vector<float>& output, std::array<dim_t, 3>& outDim) const;

  bool sampleBoundingBox(const Circle<int>& circle, const dim_t sampleSize, std::vector<float>& colorSampled, std::array<dim_t, 3>& colorSampledDim) const;

  /// applies CNN on candiates
  float filterByCNN(std::vector<float>& sampled, std::array<dim_t, 3>& colorSampledDim);

  /**
   * @brief sendDebugImage sends a debug image iff requested
   */
  void sendDebugImage();

  /// the minimal/maximal radiusRatio a dark segment should have
  const Parameter<float> seedRadiusRatioMin_;
  const Parameter<float> seedRadiusRatioMax_;
  /// the maximum Y value a dark segment should have
  const Parameter<int> seedDark_;
  /// the minimum difference to dark segments a bright neighbour pixel should have
  const Parameter<int> seedBrightMin_;
  const Parameter<int> seedBright_;
  /// the minimum amount of bright pixels that match seedBright condition
  const Parameter<int> seedBrightScore_;
  const Parameter<bool> projectFoundBalls_;

  dim_t netSampleSize_;
  std::vector<int> netConvActivation_;
  std::vector<int> netConvPooling_;
  int netFCActivation_;

  const Parameter<float> netAccuracy_;

  /// the masks/bias/amount for the convolutional layers
  std::vector<std::vector<std::vector<std::vector<std::vector<float>>>>> netConvMask_;
  std::vector<std::vector<float>> netConvBias_;
  std::vector<std::vector<float>> netNorm_;
  /// the weight/bias variables for the fully connected network
  std::vector<std::vector<std::vector<float>>> netFCWeights_;
  std::vector<std::vector<float>> netFCBias_;

  const Dependency<ImageData> imageData_;
  const Dependency<CameraMatrix> cameraMatrix_;
  const Dependency<ImageRegions> imageRegions_;
  const Dependency<FieldBorder> fieldBorder_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<CycleInfo> cycleInfo_;

  const Reference<BallState> ballState_;

  /// the generated ball
  Production<BallData> ballData_;

  /// mounts of debug images
  unsigned int candidateCount_;
  const Parameter<bool> writeCandidatesToDisk_;
  /// seed for debug image
  std::vector<Circle<int>> debugSeeds_;
  /// circles that should be drawn into the debug image
  std::vector<DebugCircle> debugCircles_;
};
