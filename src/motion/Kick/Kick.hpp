#pragma once

#include "Tools/Math/Eigen.hpp"

#include "Data/CycleInfo.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/KickConfigurationData.hpp"
#include "Data/KickOutput.hpp"
#include "Data/MotionActivation.hpp"
#include "Framework/Module.hpp"
#include "Tools/Kinematics/KinematicMatrix.h"

#include "Utils/Interpolator/Interpolator.hpp"


class Motion;

/**
 * @brief execute a dynamic kick that adapts to the current ball position
 */
class Kick : public Module<Kick, Motion>
{
public:
  /// the name of this module
  ModuleName name = "Kick";
  /**
   * @brief the Kick class
   * @param manager a reference to motion
   */
  Kick(const ModuleManagerInterface& manager);

  void cycle();

private:
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<IMUSensorData> imuSensorData_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<KickConfigurationData> kickConfigurationData_;
  const Dependency<MotionActivation> motionActivation_;
  const Dependency<MotionRequest> motionRequest_;

  Production<KickOutput> kickOutput_;

  /// whether the left or right foot is supposed to kick
  bool leftKicking_;

  /// torso offset for left kick
  const Parameter<Vector3f> torsoOffsetLeft_;
  /// tors ooffset for right kick
  const Parameter<Vector3f> torsoOffsetRight_;

  /// interpolators for all kick phases
  Interpolator waitBeforeStartInterpolator_;
  Interpolator weightShiftInterpolator_;
  Interpolator liftFootInterpolator_;
  Interpolator swingFootInterpolator_;
  Interpolator kickBallInterpolator_;
  Interpolator pauseInterpolator_;
  Interpolator retractFootInterpolator_;
  Interpolator extendFootAndCenterTorsoInterpolator_;
  Interpolator waitBeforeExitInterpolator_;

  /// an array containing all inteprolators
  const std::array<Interpolator*, 9> interpolators_ = {
      {&waitBeforeStartInterpolator_, &weightShiftInterpolator_, &liftFootInterpolator_,
       &swingFootInterpolator_, &kickBallInterpolator_, &pauseInterpolator_,
       &retractFootInterpolator_, &extendFootAndCenterTorsoInterpolator_,
       &waitBeforeExitInterpolator_}};

  /// the current interpolator id
  unsigned int currentInterpolatorID_;

  /// gyroscope filter coefficient and feedback gains
  const Parameter<float> gyroLowPassRatio_;
  const Parameter<float> gyroForwardBalanceFactor_;
  const Parameter<float> gyroSidewaysBalanceFactor_;

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
  void computeWeightShiftAnglesFromReferenceCom(const std::vector<float>& currentAngles,
                                                const Vector3f& weightShiftCom,
                                                std::vector<float>& weightShiftAngles) const;

  /**
   * @brief computeLegAnglesFromFootPose computes leg angles from foot pose
   * @param currentAngles the current angles
   * @param nextLeft2right the desired foot pose
   * @param nextAngles output parameter containing joint angles
   */
  void computeLegAnglesFromFootPose(const std::vector<float>& currentAngles,
                                    const KinematicMatrix& nextLeft2right,
                                    std::vector<float>& nextAngles) const;

  /**
   * @brief separateAngles separates left and right leg angles
   * @param left output parameter containing the left left angles
   * @param right output parameter containing the right leg angles
   * @param body angles of the whole body
   */
  void separateAngles(std::vector<float>& left, std::vector<float>& right,
                      const std::vector<float>& body) const;

  /**
   * @brief combineAngles combines left and right leg angles
   * @param result output parameter containing whole body angles
   * @param body angles of the whole body
   * @param left the left leg angles
   * @param right the right leg angles
   */
  void combineAngles(std::vector<float>& result, const std::vector<float>& body,
                     const std::vector<float>& left, const std::vector<float>& right) const;

  /**
   * @brief gyroFeedback applies gyroscope feedback to ankle roll and pitch
   * @param outputAngles output parameter containing whole body angles
   */
  void gyroFeedback(std::vector<float>& outputAngles) const;
};
