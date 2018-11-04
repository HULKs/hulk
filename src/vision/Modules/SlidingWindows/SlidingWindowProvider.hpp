#pragma once

#include "Framework/Module.hpp"

#include "Tools/Chronometer.hpp"
#include "Tools/Kinematics/ForwardKinematics.h"

#include "Data/CameraMatrix.hpp"
#include "Data/FieldColor.hpp"
#include "Data/ImageData.hpp"
#include "Data/ImageSegments.hpp"
#include "Data/SlidingWindows.hpp"

class Brain;

class SlidingWindowProvider : public Module<SlidingWindowProvider, Brain>
{
public:
  ModuleName name = "SlidingWindowProvider";
  /**
   * @brief SlidingWindowProvider provides pseudo-projected windows with scorings calculated by the
   * segmented image
   * @author Georg Felbinger
   */
  SlidingWindowProvider(const ModuleManagerInterface& manager);

  void cycle();

private:
  const Dependency<CameraMatrix> cameraMatrix_;
  const Dependency<ImageData> imageData_;
  const Dependency<ImageSegments> imageSegments_;
  const Dependency<FieldColor> fieldColor_;

  /// the minimum size of a sliding window in pixel
  const Parameter<int> minWindowSize_;
  /// distance of the sample points in meter
  const Parameter<float> samplePointDistance_;
  /// whether the calculated sliding windows should be drawn to debug image
  const Parameter<bool> debugWindows_;
  /// whether the fieldColorScores should be shown within the debug image
  const Parameter<bool> debugFieldColor_;
  /// whether the edgeScores should be printed on the debug image
  const Parameter<bool> debugEdges_;

  Production<SlidingWindows> slidingWindows_;

  /// calculates the sliding windows, only once for each camera
  std::array<bool, 2> slidingWindowConfigChanged_;
  void calculateSlidingWindows();
  /// calculates the scores (fieldColor, edgePoints) for each sliding window
  void calculateScores();
  /// draws and sends debug image
  void sendDebug();
};
