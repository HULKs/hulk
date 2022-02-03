#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/BodyPose.hpp"
#include "Data/CollisionDetectorData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/Poses.hpp"
#include "Data/StepPlan.hpp"
#include "Data/WalkGeneratorOutput.hpp"
#include "Data/WalkManagerOutput.hpp"
#include "Framework/Module.hpp"
#include "Motion/Utils/Interpolator/Interpolator.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Math/Range.hpp"


class Motion;

class WalkGenerator : public Module<WalkGenerator, Motion>, public Uni::To
{
public:
  //// the name of this module
  ModuleName name__{"WalkGenerator"};

  explicit WalkGenerator(const ModuleManagerInterface& manager);

  void cycle() override;

  void toValue(Uni::Value& value) const override;

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

  const Dependency<ActionCommand> actionCommand_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<CollisionDetectorData> collisionDetectorData_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<IMUSensorData> imuSensorData_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<Poses> poses_;
  const Dependency<StepPlan> stepPlan_;
  const Dependency<WalkManagerOutput> walkManagerOutput_;

  Production<WalkGeneratorOutput> walkGeneratorOutput_;

  /// Duration of a single step; i.e. half of a walk cycle [s]
  const Parameter<float> baseWalkPeriod_;
  /// Additional duration when walking sideways
  const Parameter<Pose> walkPeriodIncreaseFactor_;
  /// Walk hip height above ankle joint [m]
  const Parameter<float> walkHipHeight_;
  /// Base foot lift [m]
  const Parameter<float> baseFootLift_;
  /// Additional lifting as factors of forward and sideways speeds.
  const Parameter<Pose> footLiftIncreaseFactor_;
  /// Lifting of first step is changed by this factor.
  const Parameter<float> footLiftFirstStepFactor_;
  /// In which range of the walk phase can the support foot change?
  const Parameter<Range<float>> supportSwitchPhaseRange_;
  /// The maximum number of weight shift misses before emergency behavior.
  const Parameter<unsigned int> maxWeightShiftMisses_;
  /// The size of emergency sideways steps [m]
  const Parameter<float> emergencyStepSize_;
  /// How much longer than expected is a slow weight shift?
  const Parameter<float> minSlowWeightShiftRatio_;
  /// How many slow weight shifts are acceptable?
  const Parameter<unsigned int> maxSlowWeightShifts_;
  /// How long to stand after slow weight shifts were detected [s]
  const Parameter<Clock::duration> slowWeightShiftStandDelay_;
  /// The base forward offset of the torso relative to the ankles [m]
  const Parameter<float> torsoOffset_;
  /// Joint stiffness for all joints but the arms while walking
  const Parameter<float> walkLegStiffness_;
  /// Joint stiffness for all joints but the arms while standing
  const Parameter<float> standLegStiffness_;
  /// Joint stiffness for the arms joints
  const Parameter<float> armStiffness_;
  /// Arm shoulder angle [rad]
  Parameter<float> armShoulderRoll_;
  /// Factor between sideways step size and additional arm roll angles [rad/m]
  const Parameter<float> armShoulderRollIncreaseFactor_;
  /// Factor between forward foot position and arm pitch angles [rad/m]
  const Parameter<float> armShoulderPitchFactor_;
  /// The low pass ratio for the gyro. (close to 1 -> strongly filtered)
  const Parameter<float> gyroLowPassRatio_;
  /// How much are gyro measurements added to ankle joint angles to compensate falling forwards
  /// while walking?
  const Parameter<float> gyroForwardBalanceFactor_;
  /// How much are gyro measurements added to ankle joint angles to compensate falling backwards
  /// while walking?
  const Parameter<float> gyroBackwardBalanceFactor_;
  /// How much are gyro measurements added to ankle joint angles to compensate falling sideways
  /// while standing?
  const Parameter<float> gyroSidewaysBalanceFactor_;
  /// set to true to enable torso compensation
  const Parameter<bool> enableTorsoCompensation_;
  /// the gain to compensate the shifted arms (increase to lean further forward when arms are pulled
  /// back)
  const Parameter<float> armComGain_;
  /// the proportional gain to compensate the head position by shifting the torso
  const Parameter<float> speedCompensationGain_;
  /// set to true to enable the ankle controller also for stand
  const Parameter<bool> enableGyroBalanceInStand_;
  /// set to true to allow to pull back the arms in case of collision detection
  const Parameter<bool> enableCollisionReaction_;
  /// set to true to trigger collision for debug
  const Parameter<bool> triggerDebugCollision_;
  /// the duration of the arm lift motion for collision avoidance [s]
  const Parameter<float> armLiftDuration_;
  /// the duration of the motion pulling the arms close to the body [s]
  const Parameter<float> armPullTightDuration_;
  /// the minimum time the robot has to keep standing before we allow leaving [s]
  const Parameter<Clock::duration> minTimeInStandBeforeLeaving_;
  /// the maximum distance a step is considered as zero step --> walking can directly stop
  /// afterwards [m] and [deg]
  Parameter<Pose> maxDirectStandStepSize_;

  /// key frame angles for arms from armBack pose
  const Parameter<JointsArray<float>> armLiftAngles_;
  /// key frame angles for arms in collision avoidance mode
  const Parameter<JointsArray<float>> armPullTightAngles_;

  // variables to handle the state related things (walking, standing etc.)
  /// current state of the walking engine (e.g. STANDING)
  WalkState walkState_{WalkState::STANDING};
  /// the last time the walking engine walked
  Clock::time_point lastTimeWalking_;

  // speed states:
  // .. for foward direction ..
  /// Forward step size [m/step] Forward is positive.
  float forward_{0.f};
  /// The forward offset of the left foot as seen from the torso [m]
  float forwardL_{0.f};
  /// The forward offset of the right foot as seen from the torso [m]
  float forwardR_{0.f};
  /// Forward offset of the left foot when the support changed [m]
  float forwardL0_{0.f};
  /// Forward offset of the right foot when the support changed [m]
  float forwardR0_{0.f};
  // .. for sideways direction ..
  /// Sideways step size [m/step] Left is positive.
  float left_{0.f};
  /// The sideways offset of the left foot as seen from the torso [m]
  float leftL_{0.f};
  /// The sideways offset of the right foot as seen from the torso [m]
  float leftR_{0.f};
  /// The sideways offset of the left foot when the support changed [m]
  float leftL0_{0.f};
  /// The sideways offset of the right foot when the support changed [m]
  float leftR0_{0.f};
  // .. and turning
  /// Turn size in [rad/step] Anti-clockwise is positive.
  float turn_{0.f};
  /// The turn angle for both feet [rad]
  float turnRL_{0.f};
  /// The turn angle for both feet when the support changed [rad]
  float turnRL0_{0.f};

  /// The foot height offset of the left foot
  float footHeightL_{0.f};
  /// The foot height offset of the right foot
  float footHeightR_{0.f};

  /// The walk phase when the support changed
  float switchPhase_{0.f};

  // foot trajectory
  /// Maximum foot height in current step [m]
  float maxFootHeight_{0.f};
  /// Maximum foot height in previous step [m]
  float maxFootHeight0_{0.f};

  /// the status of the weight shift process (e.g. it took longer than expected)
  WeightShiftStatus weightShiftStatus_{WeightShiftStatus::WEIGHT_DID_NOT_SHIFT};
  /// The time when slow weight shifts were detected.
  Clock::time_point timeWhenSlowWeightShiftsDetected_;
  /// How often was the weight not shifted in a row?
  unsigned int weightShiftMisses_{0};
  /// How often took the weight shift significantly longer in a row?
  unsigned int slowWeightShifts_{0};

  // controller states
  /// Lowpass-filtered gyro measurements around y axis [rad/s]
  float filteredGyroX_{0.f};
  /// Lowpass-filtered gyro measurements around y axis [rad/s]
  float filteredGyroY_{0.f};

  /// compensator states
  float lastStepwiseTorsoCompensation_{0.f};

  /// Arm interpolators for collision avoidance (first and second stage)
  Interpolator<float, static_cast<std::size_t>(Joints::MAX)> armStageOneInterpolator_;
  Interpolator<float, static_cast<std::size_t>(Joints::MAX)> armStageTwoInterpolator_;
  /// the state of the arms to keep track of the currently performed arm motion
  WalkGeneratorOutput::ArmState armState_{WalkGeneratorOutput::ArmState::NORMAL};


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
   * @brief initializeStepStatesFromRequest initializes the states based on the request
   * @param forward requested step size in forward direction [m/step]
   * @param left requested step size in left direction [m/step]
   * @param turn requested step size in turn direction [rad/step]
   */
  void initializeStepStatesFromRequest(float forward, float left, float turn);
  /**
   * Run method of the walk cycle. Called while walking
   */
  void walkCycle();
  /**
   * @brief handleSupportPhaseEnd (re)sets the internal states for the beginning of a new step (e.g.
   * checks whether the weight shifted, logs the time of this event, saves end positions of legs as
   * initial conditions for the next step)
   */
  void handleSupportPhaseEnd();
  /**
   * @brief calcFoot2TorsoFromOffsets calculates the foot2torso 3D pose from given offsets
   * @param footSign the sign of the foot (1 for left foot, -1 for right foot)
   * @param footYawAngle the angle with which the foot is turned around the z-axis
   * @param legRollAngle the HipRoll angle with which the leg is rotated sideways
   * @param footForwardOffset the offset of the foot in x-direction
   * @param footHeight the height to which the foot is lifted (relative to foot height when
   * standing)
   */
  KinematicMatrix calcFoot2TorsoFromOffsets(float footSign, float turnRL, float left, float forward,
                                            float footHeight);
  /**
   * Resets the generator. Must be called whenever the control is returned to this module after
   * another module was responsible for creating the motions and once after creation.
   */
  void resetGenerator();
  /**
   * Updates the state machine of the walk state.
   */
  void determineWalkState();
  /**
   * Calculates the foot offsets of this cycle
   */
  void calcFootOffsets();
  /**
   * Calculates a new set of joint angles to let the robot walk or stand. Must be called every cycle
   * @param forward requested step size in forward direction [m/step]
   * @param left requested step size in left direction [m/step]
   * @param turn requested step size in turn direction [rad/step]
   * @param getKickFootOffset If set, provides an offset to add to the pose of the swing foot to
   *                          create a kick motion. It must be suited for the foot that actually is
   *                          the swing foot.
   */
  void calcJoints(const std::function<KinematicMatrix(const float phase)>& getKickFootOffset);

  /**
   * The method determines the forward, left, and lift offsets of both feet.
   * The method distiguishes between the swing foot and the support foot.
   * @param forwardSwing0 Forward offset of the current swing foot when the support changed [m]
   * @param forwardSupport0 Forward offset of the current support foot when the support changed [m]
   * @param forwardSwing The new forward offset of the swing foot is returned here [m]
   * @param forwardSupport The new forward offset of the support foot is returned here [m]
   * @param leftSwing0 Sideways offset of the swing foot when support changed [m]
   * @param leftSupport0 Sideways offset of the support foot when support changed [m]
   * @param leftSwing The new sideways angle of the swing foot is returned here [m]
   * @param leftSupport The new sideways angle of the support foot is returned here [m]
   * @param footHeightSwing The new lift offset of the swing foot is returned here [m]
   * @param footHeightSupport The new lift offset of the support foot is returned here [m]
   */
  void calcFootOffsets(float forwardSwing0, float forwardSupport0, float* forwardSwing,
                       float* forwardSupport, float leftSwing0, float leftSupport0,
                       float* leftSwing, float* leftSupport, float* footHeightSwing,
                       float* footHeightSupport);

  /**
   * Returns values on a parabola with f(0) = f(1) = 0, f(0.5) = 1.
   * @param f A value between 0 and 1.
   * @return The value on the parabola for "f".
   */
  static float parabolicReturn(float f); /// step function with deadTimeFraction/2 delay

  /**
   * Returns values on a parabola with f(0) = 0, f(1) = 1.
   * @param f A value between 0 and 1.
   * @return The value on the parabola for f
   */
  static float parabolicStep(float f);

  /**
   * Calculates the leg angles from given foot poses and places them inside a set of given body
   * angles
   * @param leftFoot the 3D-pose of the left foot relative to the torso
   * @param rightFoot the 3D-pose of the right foot relative to the torso
   * @param isLeftSwing true if the left foot is the swing foot
   * @param bodyAngles - out param - the body angles to place the result in
   */
  JointsArray<float> calculateBodyAnglesFromFootPoses(const KinematicMatrix& leftFoo,
                                                      const KinematicMatrix& rightFoot,
                                                      bool isLeftSwing) const;

  /**
   * Calculates a torso shift to compensate for unmodelled effects like head motion
   * @return the compensating torso shift (postive if the torso is to be shifted forward)
   */
  float getTorsoCompensationShift() const;

  /**
   * Calculates "natural" arm swing while walking to counterbalance foot swing or moves arms to the
   * back to avoid collisions
   */
  void handleArms();

  /**
   * Adds balancing adjustments to the foot angles
   */
  void balanceAdjustment();
};

inline void WalkGenerator::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::OBJECT);
  value["walkState"] << static_cast<unsigned int>(walkState_);
  value["lastTimeWalking"] << lastTimeWalking_;
  value["forward"] << forward_;
  value["forwardL"] << forwardL_;
  value["forwardR"] << forwardR_;
  value["forwardL0"] << forwardL0_;
  value["forwardR0"] << forwardR0_;
  value["left"] << left_;
  value["leftL"] << leftL_;
  value["leftR"] << leftR_;
  value["leftL0"] << leftL0_;
  value["leftR0"] << leftR0_;
  value["turn"] << turn_;
  value["turnRL"] << turnRL_;
  value["turnRL0"] << turnRL0_;
  value["footHeightL"] << footHeightL_;
  value["footHeightR"] << footHeightR_;
  value["switchPhase"] << switchPhase_;
  value["maxFootHeight"] << maxFootHeight_;
  value["maxFootHeight0"] << maxFootHeight0_;
  value["weightShiftStatus"] << static_cast<unsigned int>(weightShiftStatus_);
  value["timeWhenSlowWeightShiftsDetected"] << timeWhenSlowWeightShiftsDetected_;
  value["weightShiftMisses"] << weightShiftMisses_;
  value["slowWeightShifts"] << slowWeightShifts_;
  value["filteredGyroX"] << filteredGyroX_;
  value["filteredGyroY"] << filteredGyroY_;
  value["lastStepwiseTorsoCompensation"] << lastStepwiseTorsoCompensation_;
  value["armState"] << static_cast<unsigned int>(armState_);
}
