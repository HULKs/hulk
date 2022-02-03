#pragma once

#include "Brain/Knowledge/Position/FieldInfo.hpp"
#include "Brain/Knowledge/Position/PoseHypothesis.hpp"
#include "Brain/Knowledge/Position/PositionProvider.hpp"
#include "Data/BodyPose.hpp"
#include "Data/CameraMatrix.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/ImageData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/LandmarkModel.hpp"
#include "Data/MotionState.hpp"
#include "Data/OdometryOffset.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/RobotPosition.hpp"
#include "Framework/Module.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Math/Eigen.hpp"
#include <vector>


class Brain;

/**
 * @brief The UKFPositionKnowledge class
 */
class UKFPositionKnowledge : public Module<UKFPositionKnowledge, Brain>, public Uni::To
{
public:
  /// the name of this module
  ModuleName name__{"UKFPositionKnowledge"};
  /**
   * @brief UKFPositionKnowledge initializes the members of the UKFPositionKnowledge
   * @param manager a reference to the brain
   */
  UKFPositionKnowledge(const ModuleManagerInterface& manager);
  /**
   * @brief cycle integrates prediction and measurement into the position estimation
   */
  void cycle();
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the resulting Uni::Value
   */
  void toValue(Uni::Value& value) const;

private:
  /// the standard deviation when in initial
  const Parameter<Vector3f> sigmaInitial_;
  /// the standard deviation when penalized
  const Parameter<Vector3f> sigmaPenalized_;
  /// the filte process noise
  const Parameter<Vector3f> filterProcessNoise_;
  /// the predict process noise (will be scaled with moved distance)
  const Parameter<Vector3f> predictProcessNoiseFraction_;
  /// factor for the hypothesis selection hysteresis
  const Parameter<float> hypothesisSelectionHysteresis_;
  /// an absolute threshold for the meanEvalError_ of the PoseHypothesis
  const Parameter<float> absoluteEvalThreshold_;
  /// a relative (to the best pose) threshold for the meanEvalError_ of the PoseHypothesis
  const Parameter<float> relativeEvalThreshold_;
  /// a threshold representing the maximum distance when merging poses
  const Parameter<float> mergeRadius_;
  /// a threshold representing the maximum angleDif when merging poses
  const Parameter<float> mergeAngle_;
  /// the maximum association distance of penalty spots in penalty shoout out
  const Parameter<float> maxPSOPenaltySpotAssociationDistance_;
  /// if set to true, the hypotheses will be spread along the whole sideline
  const Parameter<bool> startAnywhereAtSidelines_;
  /// the maximum number of hypotheses
  const Parameter<unsigned int> maxNumberOfHypotheses_;
  /// if set to true, sensor resetting is allowed
  const Parameter<bool> useSensorResetting_;
  /// if set to true, the internal method for finding the center circle from lines will be used
  const Parameter<bool> useInternalCircleDetection_;
  /// if set to true, circle percepts that would be near goal support area when projected from the
  /// hypothesis will be ignored
  const Parameter<bool> ignoreCirclePerceptsNearGoalSupport_;
  /// if set to true, penalty areas without orientation are dropped (not used for updates)
  const Parameter<bool> ignorePenaltyAreasWithoutOrientation_;
  /// if set to true, multiple hypothesis are always created in PSO. Even if the gamecontroller
  /// doesn't claim the mode to be of type CompetitionPhase::GENERAL_PENALTY_KICK
  const Parameter<bool> alwaysUseMultiplePenaltyShootoutPositions_;
  /// if set to true, the striker will use measurments (thus acively localize in PSO)
  const Parameter<bool> strikerLocalizeInPSO_;
  /// if set to true hypothesis can be configured with own half and left half parameters
  const Parameter<bool> eventMode_;
  /// if set to true spawn hypothesis on own half in event mode
  const Parameter<bool> eventOwnHalf_;
  /// if set to true spawn hypothesis on left half in event mode
  const Parameter<bool> eventLeftHalf_;
  /// in blind flight the measurements are not used, the robot is only predicting its pose
  const Parameter<bool> blindFlight_;

  /// some details about the cycle time
  const Dependency<CycleInfo> cycleInfo_;
  /// the dimensions of the field, as well as some methods to check position in field coordinates
  const Dependency<FieldDimensions> fieldDimensions_;
  /// a reference to the odometry offset
  const Dependency<OdometryOffset> odometryOffset_;
  /// a reference to the camera matrix
  const Dependency<CameraMatrix> cameraMatrix_;
  /// current game controller state
  const Dependency<GameControllerState> gameControllerState_;
  /// configuration for this particular player
  const Dependency<PlayerConfiguration> playerConfiguration_;
  /// landmarks for sensor resetting / pose updates
  const Dependency<LandmarkModel> landmarkModel_;
  /// the pose of the body
  const Dependency<BodyPose> bodyPose_;
  /// the currently executed motion
  const Dependency<MotionState> motionState_;
  /// the current position of all joints (used for the current  head yaw)
  const Dependency<JointSensorData> jointSensorData_;
  /// the current camera image to figure out whether the lines were recoreded by the top or bottom
  /// camera
  const Dependency<ImageData> imageData_;

  /// the robot position that is estimated by this module
  // TODO: as soon as the particle filter is deprecated one can add the cov estimation and number of
  // hypotheses to the RobotPosition
  Production<RobotPosition> robotPosition_;
  /// the pose estimate of the last cycle
  Pose lastPose_;
  /// the timestamp of the last time the robot pose significantly jumped
  Clock::time_point lastTimeJumped_;
  /// all field lines and goal posts on the soccer field
  FieldInfo fieldInfo_;
  /// a class that can provide positions based on game situation or vision results
  PositionProvider positionProvider_;
  /// a vector of possible pose hypotheses, tracked by an UKF
  std::list<PoseHypothesis> poseHypotheses_;
  /// the best PoseHypothesis according to the evaluation
  std::list<PoseHypothesis>::iterator bestPoseHypothesisIt_;
  /// the state of the last cycle
  GameState lastState_;
  /// the penalty of the last cycle
  Penalty lastPenalty_;
  /// whether vision data should be integrated into the filter
  bool useMeasurements_;
  /// whether whether resetting is currently allowed
  bool resettingIsAllowed_;
  /// whether all hypothesis have been resetted in this cycle
  bool resettedThisCycle_;
  /// whether the robot jumped this cycle
  bool jumpedThisCycle_;
  /// true if this robot lost ground contact during this SET phase
  bool wasHighInSet_;

  /**
   * @brief updateState determines the state the localization is in
   */
  void updateState();
  /**
   * @brief preparePoseHypotheses sets up the pose hypothesis for a given situation
   * @param numberOfHypotheses the number of hypotheses to be set up
   * @param poseProviderFunc a lamdba to get the positions form
   */
  void preparePoseHypotheses(const unsigned int numberOfHypotheses, const Vector3f& sigma,
                             const std::function<Pose(unsigned int& clusterID)>& poseProviderFunc);
  /**
   * @brief odometryPredict uses odometry (from motion) to perform a ukf predict for each hypothesis
   * in poseHypotheses_
   */
  void odometryPredict();
  /**@brief linesAreValid checks whether the lines of this image can be used by checking, whether
   * the camera is currently occluded
   * @return true if the lines of this image are believed to be valid
   */
  bool linesAreValid() const;
  /**
   * @brief http://stackoverflow.com/a/1501725/2169988 (find the shortest distance between a point
   * and a line segment)
   * @param line the line to get the distance to
   * @param point a point which distance to a line is to be computed
   * @return shortest distance between point and line
   */
  float distPointToLineSegment(const Line<float>& line, const Vector2f& point) const;
  /**
   * @brief measurementUpdate uses measurements (i.e. vision data) to perform a ukf predict for each
   * hypothesis in poseHypotheses_
   */
  void measurementUpdate();
  /**
   * @brief mergeHypotheses merges matching hypothses
   */
  void mergeHypotheses();
  /**
   * @brief generateNewHypotheses generates new hypotheses (e.g. from sensor resetting)
   */
  void generateNewHypotheses();
  /**
   * @brief evaluateHypotheses checks the hypotheses in poseHypotheses_ for validity and eventually
   * eliminates them
   */
  void evaluateHypotheses();
  /**
   * @brief updateLastTimeJumped updates lastTimeJump if the robot pose changed significantly
   * @return the time stamp of the last significant (unusual) pose change
   */
  Clock::time_point updateLastTimeJumped(const Pose& currentPoseEstimate);
  /**
   * @brief publishPoseEstimate computes the effective position of the robot
   */
  void publishPoseEstimate();
};
