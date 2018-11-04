#pragma once

#include <vector>

#include <Framework/Module.hpp>

#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/GoalData.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/LandmarkModel.hpp"
#include "Data/MotionState.hpp"
#include "Data/OdometryOffset.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/RobotPosition.hpp"
#include "Tools/Time.hpp"

#include "FieldInfo.hpp"
#include "PositionParticle.hpp"
#include "PositionProvider.hpp"

class Brain;

/**
 * @brief The ParticlePositionKnowledge class
 */
class ParticlePositionKnowledge : public Module<ParticlePositionKnowledge, Brain>, public Uni::To
{
public:
  /// the name of this module
  ModuleName name = "ParticlePositionKnowledge";
  /**
   * @brief ParticlePositionKnowledge initializes the members of the ParticlePositionKnowledge
   * @param manager a reference to the brain
   */
  ParticlePositionKnowledge(const ModuleManagerInterface& manager);
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
  /**
   * @brief updateState determines the state the localization is in
   */
  void updateState();
  /**
   * @brief integrateOdometry uses odometry (from motion) to predict the particle positions
   */
  void integrateOdometry();
  /**
   * @brief integrateMeasurements uses measurements (i.e. vision data) to weight the particles
   */
  void integrateMeasurements();
  /**
   * @brief resample will modify the sample set s.t. the number of particles at a certain position
   * is proportional to its weight
   */
  void resample();
  /**
   * @brief computePosition computes the effective position of the robot
   */
  void computePosition();
  /**
   * @brief resetWeights sets all particle weights to 1 / numberOfParticles
   */
  void resetWeights();
  /**
   * @brief evaluateParticle decides whether a particle is valid or not
   */
  void evaluateParticle();
  /**
   * @brief updateLastTimeJumped updates lastTimeJump if the robot pose changed significantly
   */
  void updateLastTimeJumped();
  /**
   * @brief predictParticleMovement predicts the movement of the given particle
   * @param particle as written above
   * @param pose the movement of the robot
   */
  void predictParticleMovement(PositionParticle& particle, const Pose& pose);
  /**
   * @brief updateWithLine: Each particle is updated with a line by placing the recognized line in
   * respect of the particles position
   * @param line a line in robot coordinates as seen by the vision
   * @param projectionDistance the distance of the line center to the robot
   */
  void updateWithLine(PositionParticle& particle, const Line<float>& line,
                      const float projectionDistance);
  /**
   * @brief updateWithLandMarkPosition updates a given particle with relative landmark measurement
   * (e.g. the position of the center circle)
   * @param relativeLandmarkMeasurement the relative position of the percepted land mark
   * @param the variance of the land mark measurement
   * @param absoluteGroundTruthPosition the point on th map (ground truth), where this feature
   * should be
   */
  void updateWithLandMarkPosition(PositionParticle& particle,
                                  const Vector2f& relativeLandmarkMeasurement,
                                  const float measurementVariance,
                                  const Vector2f& absoluteGroundTruthPosition);
  /**
   * @brief updateWithGoalPosts: Each particle is updated with the goal posts by placing the
   * recognized goal post in respect of the particles position
   * @param goalPosts a vector of goal posts in robot coordinates as seen by the vision
   */
  void updateWithGoalPosts(PositionParticle& particle, const VecVector2f& goalPosts);
  /**
   * @brief angleBetweenLineVectors calculates the smallest angle between two lines (range from 0 to
   * M_PI_2)
   * @param v1 the first line vector (pointing along the line)
   * @param v2 the second line vector (pointing along the line)
   * @return the smallest angle between the two vectors (range 0 to M_PI_2)
   */
  float angleBetweenLineVectors(const Vector2f& v1, const Vector2f& v2) const;
  /**
   * @brief weightByLine calculates the weight of the particle for a percepted line (in field
   * coordinates, lower weight when percepted line matches no line on field)
   * @param line the percepted line in field coordinates
   * @param projectionDistance the distance of projection of line center point
   * @return a weight that corresponds to the probability that this particle a good position
   * estimation
   */
  float weightByLine(const Line<float>& line, const float projectionDistance) const;
  /**
   * @brief weightByGoalPost computes a weight for a seen post in assumed field coordinates
   * @param particle the particle to calculate the related weight
   * @param goalPost the seen goal post in assumed field coordinates
   * @return a weight <= 1 that is higher the more the seen post matches a post on the field
   */
  float weightByGoalPost(const Vector2f& goalPost) const;
  /// standard deviation for resampling
  const Parameter<Vector3f> sigma_;
  /// standard deviation for prediction
  const Parameter<Vector3f> sigmaPrediction_;
  /// current number of particles in use
  const Parameter<int> numberOfParticles_;
  /// the maximum amount of lines that are used to update the localization
  const Parameter<int> maxConsideredLineMeasurements_;
  /// the maximum allowed distance to associate a point with a line
  const Parameter<float> lineAssociationDistance_;
  /// the maximum allowed angle to associate a line with a field line
  Parameter<float> lineAssociationAngle_;
  /// the maximum allowed euclidean norm of the gyro measurement when using measurements
  const Parameter<float> maxGyroNormWhenMeasuring_;
  /// the fraction of particles that is replaced by sensor resetting if available
  const Parameter<float> sensorResettingFraction_;
  /// whether to transmit all particles with seen lines etc. (makes walking with the robot
  /// impossible)
  const Parameter<bool> transmitParticles_;
  /// if set to true, multiple hypothesis are always created in PSO. Even if the gamecontroller
  /// doesn't claim the mode to be of type CompetitionPhase::GENERAL_PENALTY_KICK
  const Parameter<bool> alwaysUseMultiplePenaltyShootoutPositions_;
  /// true if measurements should be used in penalty shootout by the striker
  const Parameter<bool> strikerLocalizeInPSO_;
  /// some details about the cycle time
  const Dependency<CycleInfo> cycleInfo_;
  /// goal result from vision
  const Dependency<GoalData> goalData_;
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
  /// a reference to the odometry offset
  const Dependency<OdometryOffset> odometryOffset_;
  /// a reference to the field dimensions
  const Dependency<FieldDimensions> fieldDimensions_;
  /// a reference to the IMU sensor data
  const Dependency<IMUSensorData> imuSensorData_;
  /// a reference to the body anges (used for the head yaw)
  const Dependency<JointSensorData> jointSensorData_;
  /// the robot position that is estimated by this module
  Production<RobotPosition> robotPosition_; // TODO: Refactor this?
  /// all field lines and goal posts on the soccer field
  FieldInfo fieldInfo_;
  /// a class that can provide positions based on game situation or vision results
  PositionProvider positionProvider_;
  /// the set of particles
  std::vector<PositionParticle> particles_;
  /// the next cluster ID that will be assigned
  unsigned int nextClusterID_;
  /// the estimated robot pose (position)
  Pose pose_;
  /// the pose of the robot in the last cycle
  Pose lastPose_;
  /// the timestamp of the last time the robot pose significantly jumped
  TimePoint lastTimeJumped_;
  /// the state of the last cycle
  GameState lastState_;
  /// the penalty of the last cycle
  Penalty lastPenalty_;
  /// whether vision data should be integrated into the filter
  bool useMeasurements_;
  /// whether there were actual measurements that could be used
  bool updatedWithMeasurements_;
  /// whether all the particles have been resetted in this cycle
  bool resettedWeightsThisCycle_;
  /// whether the robot jumped this cycle
  bool jumpedThisCycle_;
  /// true if this robot lost ground contact in set
  bool wasHighInSet_;
  /// the timestamp of the last used line result
  TimePoint lastLineTimestamp_;
  /// the timestamp of the last used goal result
  TimePoint lastGoalTimestamp_;
};
