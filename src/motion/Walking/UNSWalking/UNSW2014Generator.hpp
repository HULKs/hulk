#pragma once

#include "Data/BodyPose.hpp"
#include "Data/CollisionDetectorData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/MotionRequest.hpp"
#include "Data/RobotKinematics.hpp"
#include "Data/WalkGenerator.hpp"

#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Math/Range.hpp"
#include "Utils/Interpolator/Interpolator.hpp"

#include "Framework/Module.hpp"


class Motion;

class UNSW2014Generator : public Module<UNSW2014Generator, Motion>
{
public:
  //// the name of this module
  ModuleName name = "UNSW2014Generator";
  /**
   * @brief Constructor
   */
  UNSW2014Generator(const ModuleManagerInterface& manager);

  void cycle();

private:
  enum class WalkState
  {
    STANDING,
    STARTING,
    WALKING,
    STOPPING
  };

  enum class WeightShiftStatus
  {
    WEIGHT_DID_SHIFT,
    WEIGHT_DID_NOT_SHIFT,
    EMERGENCY_STEP
  };

  /// some parameters to configure the walking trajectory generation
  /// Maximum speeds in m/s and degrees/s.
  Parameter<Pose> maxSpeed_;
  /// Maximum backwards speed. Positive_; in m/s.
  const Parameter<float> maxSpeedBackwards_;
  /// Maximum acceleration of forward and sideways speed at each leg change to ratchet up/down in
  /// m/s/step.
  const Parameter<Vector2f> maxAcceleration_;
  /// (Positive) maximum deceleration of forward and sideways speed at each leg change to ratchet
  /// up/down in (m/s/step)
  const Parameter<Vector2f> maxDeceleration_;
  /// Maximum speeds in m/s and degrees/s. Slower for demo games.
  Parameter<Pose> slowMaxSpeed_;
  /// Maximum backwards speed. Positive_; in m/s. Slower for demo games.
  const Parameter<float> slowMaxSpeedBackwards_;
  /// Maximum acceleration of forward and sideways speed at each leg change to ratchet up/down in
  /// (m/s/step). Slower for demo games.
  const Parameter<Vector2f> slowMaxAcceleration_;
  /// This affects the relationship between forward and sideways.
  const Parameter<float> walkVolumeTranslationExponent_;
  /// Higher value allows turn to be higher with a high translation.
  const Parameter<float> walkVolumeRotationExponent_;
  /// Duration of a single step_; i.e. half of a walk cycle (in s)
  const Parameter<float> baseWalkPeriod_;
  /// Additional duration when walking sideways at maximum speed (in s)
  const Parameter<float> sidewaysWalkPeriodIncreaseFactor_;
  /// Walk hip height above ankle joint in m
  const Parameter<float> walkHipHeight_;
  /// Base foot lift in m.
  const Parameter<float> baseFootLift_;
  /// Additional lifting as factors of forward and sideways speeds.
  const Parameter<Vector2f> footLiftIncreaseFactor_;
  /// Lifting of first step is changed by this factor.
  const Parameter<float> footLiftFirstStepFactor_;
  /// In which range of the walk phase can the support foot change?
  const Parameter<Range<float>> supportSwitchPhaseRange_;
  /// The maximum number of weight shift misses before emergency behavior.
  const Parameter<int> maxWeightShiftMisses_;
  /// The size of emergency sideways steps in m.
  const Parameter<float> emergencyStepSize_;
  /// How much longer than expected is a slow weight shift?
  const Parameter<float> minSlowWeightShiftRatio_;
  /// How many slow weight shifts are acceptable?
  const Parameter<int> maxSlowWeightShifts_;
  /// How long to stand after slow weight shifts were detected (in ms)
  const Parameter<float> slowWaitShiftStandDelay_;
  /// How much of rotation is done by turning feet to the inside (0..1)
  const Parameter<float> insideTurnRatio_;
  /// The base forward offset of the torso relative to the ankles in m.
  const Parameter<float> torsoOffset_;
  /// Scale requests so that the executed speeds match the requested ones.
  const Parameter<Pose> speedScale_;
  /// Scale measured speeds so that they match the executed speeds.
  const Parameter<Pose> odometryScale_;
  /// Joint stiffness for all joints but the arms while walking
  const Parameter<float> walkLegStiffness_;
  /// Joint stiffness for all joints but the arms while standing
  const Parameter<float> standLegStiffness_;
  /// Joint stiffness for the arms joints
  const Parameter<float> armStiffness_;
  /// Arm shoulder angle in radians.
  Parameter<float> armShoulderRoll_;
  /// Factor between sideways step size (in m) and additional arm roll angles.
  const Parameter<float> armShoulderRollIncreaseFactor_;
  /// Factor between forward foot position (in m) and arm pitch angles.
  const Parameter<float> armShoulderPitchFactor_;
  /// The low pass ratio for the gyro. (close to 1 -> strongly filtered)
  const Parameter<float> gyroLowPassRatio_;
  /// The low pass ratio for the accelermeter. (close to 1 -> strongly filtered)
  const Parameter<float> accelerometerLowPassRatio_;
  /// How much are gyro measurements added to ankle joint angles to compensate falling forwards
  /// while walking?
  const Parameter<float> gyroForwardBalanceFactor_;
  /// How much are gyro measurements added to ankle joint angles to compensate falling backwards
  /// while walking?
  const Parameter<float> gyroBackwardBalanceFactor_;
  /// How much are gyro measurements added to ankle joint angles to compensate falling sideways
  /// while standing?
  const Parameter<float> gyroSidewaysBalanceFactor_;
  /// Ratio between distance to target and speed to walk with if it cannot be reached in a single
  /// step.
  const Parameter<float> targetModeSpeedFactor_;
  /// set to true to consider return offset set planning
  const Parameter<bool> enableReturnOffset_;
  /// set to true to enable torso compensation
  const Parameter<bool> enableTorsoCompensation_;
  /// the proportional gain to compensate the head position by shifting the torso
  const Parameter<float> headComGain_;
  /// the gain to compensate the shifted arms (increase to lean further forward when arms are pulled
  /// back)
  const Parameter<float> armComGain_;
  /// the proportional gain to compensate the head position by shifting the torso
  const Parameter<float> speedCompensationGain_;
  /// the proportional gain to compensate for forward accelerating
  const Parameter<float> accelerationCompensationGain_;
  /// set to true to enable the ankle controller also for stand
  const Parameter<bool> enableGyroBalanceInStand_;
  /// set to true to allow to pull back the arms in case of collision detection
  const Parameter<bool> enableCollisionReaction_;
  /// set to true to trigger collision for debug
  Parameter<bool> triggerDebugCollision_;
  /// the duration of the arm lift motion (for collision avoidance; in seconds)
  const Parameter<float> armLiftDuration_;
  /// the duration of the motion pulling the arms close to the body for (for collision avoidance; in
  /// seconds)
  const Parameter<float> armPullTightDuration_;

  /// dependencies from other modules
  const Dependency<BodyPose> bodyPose_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<IMUSensorData> imuSensorData_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<RobotKinematics> robotKinematics_;
  const Dependency<CollisionDetectorData> collisionDetectorData_;
  const Dependency<MotionRequest> motionRequest_;
  /// the production of this module
  Production<WalkGenerator> walkGenerator_;
  /// variables to handle the state related things (walking, standing etc.)
  WalkState walkState_;

  /// speed states:
  /// .. for foward direction ..
  /// Forward speed in m/step. Forward is positive.
  float forward_;
  /// The forward speed of the previous step.
  float lastForward_;
  /// The forward offset of the left foot (in m)
  float forwardL_;
  /// The forward offset of the right foot (in m)
  float forwardR_;
  /// Forward offset of the left foot when the support changed (in m)
  float forwardL0_;
  /// Forward offset of the right foot when the support changed (in m)
  float forwardR0_;
  /// .. for sideways direction ..
  /// Sideways speed in m/step. Left is positive.
  float left_;
  /// Sideways speed in for previous step m/s. Left is positive.
  float lastLeft_;
  /// The sideways angle of the left foot (in radians)
  float leftL_;
  /// The sideways angle of the right foot (in radians)
  float leftR_;
  /// .. and turning
  /// Turn speed in radians/step. Anti-clockwise is positive.
  float turn_;
  /// The turn angle for both feet (in radians)
  float turnRL_;
  /// The turn angle for both feet when the support changed (in radians)
  float turnRL0_;

  /// upper body trajectory
  /// Recovery angle for side stepping (in radians)
  float swingAngle_;
  /// The walk phase when the support changed.
  float switchPhase_;

  /// foot trajectory
  ///// Maximum foot height in current step (in m)
  float maxFootHeight_;
  /// Maximum foot height in previous step (in m)
  float maxFootHeight0_;

  /// the status of the weight shift process (e.g. did it take longer than expected)
  WeightShiftStatus weightShiftStatus_;
  /// The time when slow weight shifts were detected.
  TimePoint timeWhenSlowWeightShiftsDetected_ = TimePoint(0);
  /// How often was the weight not shifted in a row?
  int weightShiftMisses_;
  /// How often took the weight shift significantly longer in a row?
  int slowWeightShifts_;

  /// controller states
  /// Lowpass-filtered gyro measurements around y axis (in radians/s)
  float filteredGyroX_;
  /// Lowpass-filtered gyro measurements around y axis (in radians/s)
  float filteredGyroY_;
  /// the lowpass filtered acceleormeter for acceleration feedback
  Vector3f filteredAccelerometer_;

  /// odometry
  /// The value of "forwardL" in the previous cycle. For odometry calculation.
  float prevForwardL_;
  /// The value of "forwardR" in the previous cycle. For odometry calculation.
  float prevForwardR_;
  /// The value of "leftL" in the previous cycle. For odometry calculation.
  float prevLeftL_;
  /// The value of "leftR" in the previous cycle. For odometry calculation.
  float prevLeftR_;
  /// The value of "turn" in the previous cycle. For odometry calculation.
  float prevTurn_;
  /// some state for the odometry estimate
  Vector2f lastProjectedTorso2Support_;
  /// the last torso shift used as odometry estimate at support change time
  Vector2f lastProjectedTorsoShift_;

  /// compensator states
  float lastStepwiseTorsoCompensation_;

  /// Arm interpolators for collision avoidance (first and second stage)
  Interpolator armInterpolator1_;
  Interpolator armInterpolator2_;
  /// Next arm angles to be applied to the walking generator
  std::vector<float> nextArmAngles_;
  /// key frame angles for arms from ready pose
  std::vector<float> readyArmAngles_;
  /// key frame angles for arms from armBack pose
  std::vector<float> armLiftAngles_;
  /// key frame angles for arms in collision avoidance mode
  std::vector<float> armPullTightAngles_;
  /// the state of the arms to keep track of the currently performed arm motion
  WalkGenerator::ArmState armState_;


  /**
   * Filters all the sensor data that is used for feedback
   */
  void filterSensorData();

  /**
   * Calculate the torso compensation depending on step-values
   * @return the torso compensation shift
   */
  float getStepwiseTorsoCompensation() const;
  /**
   * Calculate the torso compensation depending on cycle-values
   * @return the torso compensation shift
   */
  float getCyclewiseTorsoCompensation() const;
  /**
   * @brief initializeStepStatesFromRequest initializes the the states (like forward_, left_ and
   * turn_) based on the the reuqest (speed, target etc.)
   *
   * @param speed The speed or step size to walk with. If everything is zero, the robot stands.
   * @param target The target to walk to if in target mode.
   * @param walkPathGradient the current direction we want to walk to (calculated by the
   * MotionPlanner)
   * @param walkMode How are speed and target interpreted?
   */
  void initializeStepStatesFromRequest(const Pose& speed, const Pose& target,
                                       const Pose& walkPathGradient,
                                       WalkGenerator::WalkMode walkMode);
  /**
   * @brief handlSupportPhaseEnd (re)sets the internal states for the beginning of a new step
   *        (e.g. checks whether the weigh shifted, logs the time of this event, saves end positions
   * of legs as initial conditions fo the next step)
   * @return true if the support foot actually changed
   */
  bool handleSupportPhaseEnd();
  /**
   * @brief calcFoot2TorsoFromOffsets calculates the foot2torso 3D pose from given (angle and
   * position) offsets
   *
   * @param footSign the sign of the foot (1 for left foot, -1 for right foot)
   * @param footYawAngle the angle with which the foot is turned around the z-axis
   * @param legRollAngle the HipRoll angle with which the leg is rotated sideways
   * @param footForwardOffset the offset of the foot in x-direction
   * @param footHeight the height to which the foot is lifted (relative to foot height when
   * standing)
   */
  KinematicMatrix calcFoot2TorsoFromOffsets(const float footSign, const float footYawAngle,
                                            const float legRollAngle, const float footForwardOffset,
                                            const float footHeight);
  /**
   * Initializes the generator. Must be called whenever the control is returned to this module after
   * another one was responsible for creating the motions. Must also be called once after creation.
   */
  void resetGenerator();
  /**
   * Calculates a new set of joint angles to let the robot walk or stand. Must be called every 10
   * ms.
   * @param speed The speed or step size to walk with. If everything is zero, the robot stands.
   * @param target The target to walk to if in target mode.
   * @param walkPathGradient the direction and requested speed in all directions
   * @param walkMode How are speed and target interpreted?
   * @param getKickFootOffset If set, provides an offset to add to the pose of the swing foot to
   *                          create a kick motion. It must be suited for the foot that actually is
   *                          the swing foot.
   */
  void calcJoints(const Pose& speed, const Pose& target, const Pose& walkPathGradient,
                  WalkGenerator::WalkMode walkMode,
                  const std::function<KinematicMatrix(const float phase)>& getKickFootOffset);

  /**
   * The method determines the forward, left, and lift offsets of both feet.
   * The method distiguishes between the swing foot and the support foot.
   * @param swingFootSign A sign based on the swingFoot (1 : left is swing foot, -1 right is swing
   * foot).
   * @param forwardSwing0 Forward offset of the current swing foot when the support changed (in m).
   * @param forwardSupport0 Forward offset of the current support foot when the support changed (in
   * m).
   * @param forwardSwing The new forward offset of the swing foot is returned here (in m).
   * @param forwardSupport The new forward offset of the support foot is returned here (in m).
   * @param leftSwing The new sideways angle of the swing foot is returned here (in radians).
   * @param leftSupport The new sideways angle of the support foot is returned here (in radians).
   * @param footHeightSwing The new lift offset of the swing foot is returned here (in m).
   * @param footHeightSupport The new lift offset of the support foot is returned here (in m).
   */
  void calcFootOffsets(const float swingFootSign, const float forwardSwing0,
                       const float forwardSupport0, float& forwardSwing, float& forwardSupport,
                       float& leftSwing, float& leftSupport, float& footHeightSwing,
                       float& footHeightSupport);

  /**
   * Determines the motion of the robot since the previous frame based on planned steps.
   * @param isLeftSwingFoot Is the left foot the current swing foot?
   * @return The offset in m and radians.
   */
  Pose calcPredictedOdometryOffset(bool isLeftSwingFoot);

  /**
   * Determines the projected torso position relative to the support foot
   * @param isLeftSwingFoot true if the left foot is the swinging foot
   * @return the projected torso position relative to the support foot
   */
  Vector2f getProjectedTorso2Support(bool isLeftSwingFoot) const;

  /**
   * Determines the motion of the robot sicne the previous frame based on the measured steps.
   * @param isLeftSwingFoot Is the left foot the current swing foot
   * @param supportChangedThisLastCycle true if the support changed within this cycle
   * @return The offest in m and radians
   */
  Pose calcMeasuredOdometryOffset(bool isLeftSwingFoot, bool supportChangedThisCycle);

  /**
   * Return a measure for how "big" the requested motion is, i.e. the "walk volume".
   * This is used to limit the requested motion to keep the steps executable.
   * @param forward Forward speed as a ratio of the maximum forward speed.
   * @param left Sideways speed as a ratio of the maximum sideways speed.
   * @param turn Turn speed as a ratio of the maximum turn speed.
   * @return The walk volume.
   */
  float calcWalkVolume(const float forward, const float left, const float turn) const;

  /**
   * Limit the requested motion to keep the steps executable. The request
   * is clamped to the surface of an elipsoid.
   * @param maxSpeed The maximum speeds allowed.
   * @param maxSpeedBackwards The maximum speed when walking backwards.
   * @param forward The forward speed in m/s will be clamped im necessary.
   * @param left Sideways speed in m/s will be clamped im necessary.
   * @param turn Turn speed in radians/s will be clamped im necessary.
   * @return Were the parameters actually clamped?
   */
  bool ellipsoidClampWalk(const Pose& maxSpeed, float maxBackwardsSpeed, float& forward,
                          float& left, float& turn) const;

  /**
   * Returns values on a parabola with f(0) = f(1) = 0, f(0.5) = 1.
   * @param f A value between 0 and 1.
   * @return The value on the parabola for "f".
   */
  float parabolicReturn(float) const; /// step function with deadTimeFraction/2 delay

  /**
   * Returns values on a parabola with f(0) = 0, f(period) = 1.
   * @param time A value between 0 and "period".
   * @param period The duration of a period.
   * @return The value on the parabola for "time".
   */
  float parabolicStep(float time, float period) const;

  /**
   * Calculates the leg angles from given foot poses and places them inside a set of given body
   * angles
   * @param leftFoot the 3D-pose of the left foot relative to the torso
   * @param rightFoot the 3D-pose of the right foot relative to the torso
   * @param isLeftSwing true if the left foot is the swing foot
   * @param bodyAngles - out param - the body angles to place the result in
   */
  void calculateBodyAnglesFromFootPoses(const KinematicMatrix& leftFoo,
                                        const KinematicMatrix& rightFoot, const bool isLeftSwing,
                                        std::vector<float>& bodyAngles) const;

  /**
   * Calculates a torso shift to compensate for unmodelled effects like head motion (and maybe later
   * also Acceleration)
   * @return the compensating torso shift (postive if the torso is to be shifted forward)
   */
  float getTorsoCompensationShift() const;

  /**
   * Calculates "natural" arm swing while walking to counterbalance foot swing or moves arms to the
   * back to avoid collisions
   */
  void handleArms();

  /**
   * TODO:
   * Reset the arm interpolator to target angles for collision situations
   */
  std::vector<float> getCurrentArmAngles() const;

  /**
   * TODO:
   * Apply calculated arm angles to the walk generator
   */
  void setArmAngles(const std::vector<float>& armAngles);
};
