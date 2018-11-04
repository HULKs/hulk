#pragma once
#include "Tools/Math/Eigen.hpp"

#include "BehaviorParameters.hpp"
#include "Data/BallSearchPosition.hpp"
#include "Data/BallState.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/HeadMotionOutput.hpp"
#include "Data/HeadPositionData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/MotionState.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/RobotKinematics.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Framework/Module.hpp"
#include "Knowledge/Position/FieldInfo.hpp"

class Brain;

enum LookAroundState
{
  INITIAL,
  GOING_RIGHT,
  GOING_LEFT,
  GOING_MIDDLE,
};

struct PointOfInterest
{
  PointOfInterest(float x, float y, float w)
    : position(x, y)
    , weight(w)
  {
  }
  PointOfInterest(Vector2f p, float w)
    : position(p)
    , weight(w)
  {
  }

  Vector2f position;
  float weight;
};

/**
 * @brief HeadPositionProvider A module to provide head positions for active vision purposes.
 *        This module is calculates head positions that are usefull for the purpose of ball tracking
 * and localization
 */
class HeadPositionProvider : public Module<HeadPositionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "HeadPositionProvider";
  HeadPositionProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  const Dependency<BallState> ballState_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<HeadMotionOutput> headMotionOutput_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<CycleInfo> cycleInfo_;

  Production<HeadPositionData> headPositionData_;

  /// The vector that includes all the points that are interesting for the localization
  std::vector<PointOfInterest> absolutePOIs_;
  /// States for the look around state machine
  LookAroundState lastLookAroundState_;
  LookAroundState nextLookAroundState_;

  /// Resting time for the look around state machine
  const Parameter<float> timeToRest_;
  /// head yaw max
  Parameter<float> yawMax_;
  /// max yaw angle to keep a target on the image
  Parameter<float> keepTargetOnImageMaxAngle_;
  // Tolerance to effectivly reach a requested position
  const Parameter<float> targetPositionTolerance_;
  /// Resting positions for the look around state machine
  HeadPosition outerPositionLeft_;
  HeadPosition outerPositionRight_;
  HeadPosition innerPosition_;
  /**
   * Fills the private vecotr absolutePOIs with interesting points
   */
  void fillInterestingLocalizationPoints();
  /**
   * Calculate the head position where the robot cann see the most POIs
   * @return the calculated head position
   */
  HeadPosition calculateLocalizationHeadPosition();
  /**
   * Method to calculate the best head position to look at the ball and POIs
   * @return the calculated head position
   */
  HeadPosition calculateBallAndLocalizationHeadPosition();
  /**
   * Method to calculate head positions to look around the ball
   */
  HeadPosition calculateLookAroundBallHeadPositions();
  /**
   * Calculates the head position to look around
   * @param angle the angle the robot looks around from
   */
  HeadPosition calculateLookAroundHeadPositions(float yawMax = 119, float angle = 0);
  /**
   * Calculates the head position to look directly at the ball
   */
  HeadPosition calculateBallTrackHeadPosition();
  /**
   * Creates a set of head positions
   * @param sampleSize the number of HeadPositions to generate
   * @param middleHeadPosition the samples will be generated around this position
   * @param yawMax the maximum yaw angle for each direction
   * @param sampleHeadPositions the vector with the resulting positions
   */
  void createSampleHeadPositions(const int sampleSize, const HeadPosition middleHeadPosition,
                                 const float yawMax,
                                 std::vector<HeadPosition>& sampleHeadPositions);
  /**
   * Iterates over all POIs and calls the score function for every sample
   * @param sampleHeadPositions the samples which shall be evaluated
   * @param bestHeadPosition the resulting best HeadPosition
   * @return returns the score for the curent HeadPosition
   */
  HeadPosition evaluateHeadPositions(std::vector<HeadPosition>& sampleHeadPositions,
                                     HeadPosition& bestHeadPosition);
  /**
   * Creates Debug output
   * */
  void sendDebug(HeadPosition& chosenHeadPosition);
  /**
   * Calculates the score for one HeadPositon
   * @param relativePositon the POI relative to the robot which shall be taken into account for the
   * scoring
   * @param hp the HeadPosition that shall be scored
   * @param debug only for debug purposes will be deleted later on
   */
  bool calculateScore(const PointOfInterest& relativePosition, HeadPosition& hp);
};
