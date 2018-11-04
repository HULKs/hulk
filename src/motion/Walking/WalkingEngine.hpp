#pragma once

// std
#include <fstream>
#include <vector>

// tuhhSDK includes
#include <Data/BodyPose.hpp>
#include <Data/IMUSensorData.hpp>
#include <Data/JointSensorData.hpp>
#include <Data/MotionActivation.hpp>
#include <Data/MotionPlannerOutput.hpp>
#include <Data/MotionRequest.hpp>
#include <Data/ObstacleData.hpp>
#include <Data/RobotKinematics.hpp>
#include <Data/WalkingEngineStandOutput.hpp>
#include <Data/WalkingEngineWalkOutput.hpp>
#include <Framework/Module.hpp>
#include <Modules/NaoProvider.h>
#include <Tools/Kinematics/KinematicMatrix.h>
#include <Tools/Math/KalmanFilter.h>
#include <Tools/Math/Pose.hpp>

#include "Pendulum.hpp"

class Motion;

/**
 * @brief The WalkingEngine class uses a Pendulum object for generating a walk
 * to a specified target position
 */
class WalkingEngine : public Module<WalkingEngine, Motion>
{
public:
  /// the name of this module
  ModuleName name = "WalkingEngine";
  /**
   * @brief Constructor
   */
  WalkingEngine(const ModuleManagerInterface& manager);

  void cycle();

private:
  /**
   * @brief Starts the walking motion
   */
  void start();

  /**
   * @brief Sends a stop request to the robot. The robot will stop in a safe
   * mode. Therefore a result is given to the queue when stopped.
   * @param gracefully if true, the robot will walk on until it is slow enough to stop safely
   */
  void stop(bool gracefully);

  /**
   * Returns the change of the torso matrix between the last step and
   * the current step.
   * @return The change of the torso matrix in x, y and alpha direction
   */
  Pose getTorsoMatrixChange();

  /// compute the leg angles relative to the center of mass
  void computeLegAngles2Com(std::vector<float>& bodyAngles, const ComPosition& comCommand, const float angleL, const float angleR,
                            bool computeForStand = false);

  void disconnect();    // disconnect from motion cycle
  void updateStates();  // update states of models
  void measure();       // measurement of com
  void updateError();   // update of the measurement error
  void updateOffset();  // update of the offset
  void updateRequest(); // update of the walk request
  void countPose();
  void reportOdometry();                                     // report the torso shift of the last cycle
  void pushOdometryUpdate(const Vector2f& torsoShift);       // push odometry to resultQueue_
  void applyAnkleController(std::vector<float>& bodyAngles); // applies a given set of bodyAngles;
  void generateStandAngles();                                // generates the standAngles_ to match the walking pose

  const Parameter<Vector3f> positionOffsetLeft_;
  const Parameter<Vector3f> positionOffsetRight_;
  Parameter<float> torsoAngle_;
  const Parameter<float> walkStiffness_;
  const Parameter<float> hipCorrectionY_;
  Parameter<float> angleOffsetLeft_;
  Parameter<float> angleOffsetRight_;
  const Parameter<float> linearVel_;
  const Parameter<float> periodDuration_;
  Parameter<float> rotationAngleLimit_;
  Parameter<bool> kickInWalk_;
  const Parameter<Vector2f> kalmanQ_;
  const Parameter<Vector2f> kalmanR_;
  const Parameter<float> lowPassAlphaAnkle_;
  const Dependency<MotionActivation> motionActivation_;
  /// The output of the motion planner, needed to pass on to the pendulum and subsequently to the step planner
  const Dependency<MotionPlannerOutput> motionPlannerOutput_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<IMUSensorData> imuSensorData_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<RobotKinematics> robotKinematics_;
  Production<WalkingEngineWalkOutput> walkingEngineWalkOutput_;
  Production<WalkingEngineStandOutput> walkingEngineStandOutput_;

  Pendulum pendulum_;       // The pendulum model used for predictions
  ComPosition measuredCom_; // The measured com relative to both feet
  ComPosition errorCom_;    // The predicted measurement error (kalman)
  ComPosition comCommand_;  // The desired positions sent to the feet
  float angleL_;            // The desired angle for the left leg (around z-axis)
  float angleR_;            // The desired angle for the right leg
  float ankleAccumulator_;  // Accumulator to apply a low pass filter on the ankle angle updated with GyroData.y

  ComPosition lastComs_[4]; // The 4 last commands sent
  int lastComID_;           // The last id used for comparison

  ComOffset comOffset_; // The offset calculated from the error

  /// Walktypes used for startup and stop
  WalkingType currentWalkType_;
  WalkingType nextWalkType_;

  /// vector for body angles
  std::vector<float> walkAngles_;
  std::vector<float> standAngles_;

  /// flag to indicate that starting from stand
  bool fromStand_;
  bool startStepping_;
  bool active_;
  bool countPose_;

  /// filter
  KalmanFilter kalmanY_;
  KalmanFilter kalmanX_;

  /// slot
  int poseCountFinish_;
  supportFoot lastSupport_;

  /// odometry
  Vector2f projectedTorsoPosition_;
  Vector2f lastShift_;

  Vector2f getProjectedTorsoPostion();
};
