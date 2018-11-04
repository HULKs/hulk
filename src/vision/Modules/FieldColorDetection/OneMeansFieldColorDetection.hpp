#pragma once

#include "Framework/Module.hpp"
#include "Tools/Storage/Image.hpp"
#include "Tools/Storage/UniValue/UniValue.h"

#include "Data/CameraMatrix.hpp"
#include "Data/FieldColor.hpp"
#include "Data/ImageData.hpp"

class Brain;

/**
 * @brief The FieldColorDetection class finds the color of the field in the current image.
 *
 * @author Georg Felbinger
 */
class OneMeansFieldColorDetection : public Module<OneMeansFieldColorDetection, Brain>
{
public:
  /// the name of this module
  ModuleName name = "OneMeansFieldColorDetection";
  OneMeansFieldColorDetection(const ModuleManagerInterface& manager);
  void cycle();

private:
  struct FieldColorCluster
  {
    Vector2f mean;
    int yThresh;
  };
  /// determines the initial guess using initialStep() and saves it to config
  Parameter<bool> calculateInitialGuess_;
  /// the initial guess
  const Parameter<Vector2f> initialGuessTop_;
  const Parameter<Vector2f> initialGuessBottom_;
  /// threshold for y, since cb/cr channels are random on high y
  const Parameter<float> thresholdY_;
  /// the maximal distance from (y)uv origin
  const Parameter<int> thresholdUV_;
  /// the stepsize when sampling the image
  const int sampleRate_;
  /// the image that is currently being processed
  const Dependency<ImageData> imageData_;
  /// a reference to the camera matrix
  const Dependency<CameraMatrix> cameraMatrix_;
  /// the result of the field color detection
  Production<FieldColor> fieldColor_;
  /// Y position of the horizon
  int horizonY_;
  /// debug image counter
  unsigned int counter_ = 0;
  /// list of cameras, whether the initial guess has to be recalculated
  bool updateInitialGuessTop_;
  bool updateInitialGuessBottom_;
  /// Calculates the initial guess of (cb,cr)
  Vector2f initialStep(const Image422& image, const int yThresh, const int startY) const;
  /// Updates the cluster by moving the mean to the mean of the samples within the old cluster
  FieldColorCluster updateStep(const Image422& image, const FieldColorCluster initCluster,
                               const int maxDist, const int startY);
  /// Sends debug image and results of (cb,cr)
  void sendImageForDebug(const Image422& image);
};
