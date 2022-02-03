#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/KickConfigurationData.hpp"
#include "Data/KickOutput.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/Poses.hpp"
#include "Framework/Module.hpp"
#include "Motion/Utils/Interpolator/Interpolator.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/KinematicMatrix.hpp"


class Motion;

/**
 * @brief execute a dynamic kick that adapts to the current ball position
 */
class Kick : public Module<Kick, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"Kick"};
  /**
   * @brief the Kick class
   * @param manager a reference to motion
   */
  explicit Kick(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  const Dependency<ActionCommand> actionCommand_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<IMUSensorData> imuSensorData_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<KickConfigurationData> kickConfigurationData_;
  /// a reference to the motion activation of last cycle
  const Reference<MotionActivation> motionActivation_;
  const Dependency<Poses> poses_;

  Production<KickOutput> kickOutput_;

  /// torso offset for left kick
  const Parameter<Vector3f> torsoOffsetLeft_;
  /// tors ooffset for right kick
  const Parameter<Vector3f> torsoOffsetRight_;
  /// gyroscope filter coefficient and feedback gains
  const Parameter<float> gyroLowPassRatio_;
  const Parameter<float> gyroForwardBalanceFactor_;
  const Parameter<float> gyroSidewaysBalanceFactor_;

  /// whether the left or right foot is supposed to kick
  bool leftKicking_{true};
  /// interpolators for all kick phases
  using JointInterpolator = Interpolator<Clock::duration, static_cast<std::size_t>(Joints::MAX)>;
  JointInterpolator waitBeforeStartInterpolator_;
  JointInterpolator weightShiftInterpolator_;
  JointInterpolator liftFootInterpolator_;
  JointInterpolator kickAccelerationInterpolator_;
  JointInterpolator kickConstantInterpolator_;
  JointInterpolator kickDecelerationInterpolator_;
  JointInterpolator retractFootInterpolator_;
  JointInterpolator extendFootAndCenterTorsoInterpolator_;
  JointInterpolator waitBeforeExitInterpolator_;
  /// an array containing all inteprolators
  const std::array<JointInterpolator*, 9> interpolators_ = {
      {&waitBeforeStartInterpolator_, &weightShiftInterpolator_, &liftFootInterpolator_,
       &kickAccelerationInterpolator_, &kickConstantInterpolator_, &kickDecelerationInterpolator_,
       &retractFootInterpolator_, &extendFootAndCenterTorsoInterpolator_,
       &waitBeforeExitInterpolator_}};
  /// the current interpolator id
  std::size_t currentInterpolatorID_;
  // filtered gyroscope values
  Vector2f filteredGyro_;

  /**
   * @brief resetInterpolators resets all interpolators
   * @param kickConfiguration the configuration of the kick
   * @param torsoOffset the torso offset used for the kick
   */
  void resetInterpolators(const KickConfiguration& kickConfiguration, const Vector3f& torsoOffset);
  /**
   * @brief computeWeightShiftAnglesFromReferenceCom computes angles from a reference CoM
   * @param currentAngles the current angles
   * @param weightShiftCom the desired CoM
   * @weightShiftAngles output parameter containing joint angles
   */
  JointsArray<float>
  computeWeightShiftAnglesFromReferenceCom(const JointsArray<float>& currentAngles,
                                           const Vector3f& weightShiftCom) const;
  /**
   * @brief computeLegAnglesFromFootPose computes leg angles from foot pose
   * @param currentAngles the current angles
   * @param nextLeft2right the desired foot pose
   * @param nextAngles output parameter containing joint angles
   */
  JointsArray<float> computeLegAnglesFromFootPose(const JointsArray<float>& currentAngles,
                                                  const KinematicMatrix& nextLeft2right) const;
  /**
   * @brief gyroFeedback applies gyroscope feedback to ankle roll and pitch
   * @param outputAngles output parameter containing whole body angles
   */
  void gyroFeedback(JointsArray<float>& outputAngles) const;

  /**
   * Returns values on a parabola with f(0) = 0, f(1) = 1.
   * @param f A value between 0 and 1.
   * @return The value on the parabola for f
   */
  static float parabolicStep(float f);

  /**
   * Returns values on a parabola with f(0) = 0, f(1) = 1.
   * @param f A value between 0 and 1.
   * @return The value on the parabola for f
   */
  static float parabolicPositiveStep(float f);

  /**
   * Returns values on a parabola with f(0) = 0, f(1) = 1.
   * @param f A value between 0 and 1.
   * @return The value on the parabola for f
   */
  static float parabolicNegativeStep(float f);
};
