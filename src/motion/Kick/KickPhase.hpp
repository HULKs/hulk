#pragma once

#include "Framework/Module.hpp"
#include "Modules/NaoProvider.h"
#include "Tools/Kinematics/KinematicMatrix.h"

#include "Utils/DynamicMovementPrimitive/DynamicMovementPrimitive.hpp"
#include "Utils/Interpolator/Interpolator.hpp"


class KickPhaseHelper
{
public:
  KickPhaseHelper(const ModuleBase& module);

  void resetStraightKick(const bool leftKicking, const Vector2f ballSource, const Vector2f ballDestination, const std::vector<float> anglesAtKickRequest);

  template <typename T>
  /**
   * @brief make sure given value is within specified limits
   * @tparam T a template
   * @param value the value that is to be limited
   * @param min the lower limit
   * @param max the upper limit
   * @return value within limits
   */
  T clamp(const T value, const T min, const T max);

  /**
   * @brief merge leg angles with full body angles
   * @param result the resulting angles
   * @param body the angles the leg angles should be combined with
   * @param left left leg angles
   * @param right right leg angles
   */
  void combineAngles(std::vector<float>& result, const std::vector<float>& body, const std::vector<float>& left, const std::vector<float>& right);

  /**
   * @brief extract leg angles from full body angles
   * @param left left leg angles
   * @param right right leg angles
   * @param body the angles the leg angles should be extracted from
   */
  void separateAngles(std::vector<float>& left, std::vector<float>& right, const std::vector<float>& body);

  Vector3f getLiftPosition();

  Vector3f getSwingPosition();

  void setPreviousAngles(const std::vector<float>& previousAngles);

  void getPreviousAngles(std::vector<float>& previousAngles);

  std::vector<float> getComSingleSupport();

  std::vector<float> getComExtendAndCenter();

private:
  std::vector<float> previousAngles_;

  std::vector<float> comSingleSupport_;
  const Parameter<std::vector<float>> comSingleSupportLeftKicking_;
  const Parameter<std::vector<float>> comSingleSupportRightKicking_;
  std::vector<float> comExtendAndCenter_;
  const Parameter<std::vector<float>> comExtendAndCenterLeftKicking_;
  const Parameter<std::vector<float>> comExtendAndCenterRightKicking_;

  Vector3f liftPosition_;
  Vector3f swingPosition_;
  const Parameter<float> liftPositionModifier_;
  const Parameter<float> swingPositionModifier_;
  const Parameter<float> liftHeight_;
  const Parameter<float> swingHeight_;
  const Parameter<Vector2f> liftMin_;
  const Parameter<Vector2f> liftMax_;
  const Parameter<Vector2f> swingMin_;
  const Parameter<Vector2f> swingMax_;
};

class KickPhase
{
public:
  KickPhase(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration);

  /// enum for kick motion phases
  enum class Phase
  {
    INACTIVE,
    TO_READY,
    BALANCE,
    LIFT,
    SWING,
    RETRACT,
    EXTEND_AND_CENTER,
    WAIT,
    CATCH_FALLEN,
    MOTION_FILE
  };

  /**
   * @brief joint angles are calculated that result in a certain position of the CoM with respect to the right foot. left2right remains approximately the same
   * @param bodyAngles the angles that achieve the desired matrices
   * @param com2rightDesired the desired com position with respect to the right foot
   * @param left2rightDesired the desired left foot position with respect to the right foot
   * @param torsoRoll the desired angle between torso and right foot (i. e. the ground)
   */
  virtual void computeLegAngles(std::vector<float>& bodyAngles, const KinematicMatrix com2rightDesired, const KinematicMatrix left2rightDesired,
                                const float torsoRoll);

  /// kick phase helper object
  KickPhaseHelper& kickPhaseHelper_;

protected:
  unsigned int duration_;
  const Parameter<float> torsoRoll_;
  const Parameter<float> liftAnklePitch_;
  const Parameter<float> swingAnklePitch_;
};

/// to initialize the kick motion, the ready pose is attained
class ToReady : public KickPhase
{
public:
  ToReady(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration);

  void reset(const std::vector<float>& previousAngles);

  void getAngles(std::vector<float>& bodyAngles, const unsigned int dt);

  bool finished();

private:
  Interpolator interpolator_;
};

/// shift the CoM so that it is inside the hull of the support foot
class Balance : public KickPhase
{
public:
  Balance(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration);
  void reset(const std::vector<float>& previousAngles);
  void getAngles(std::vector<float>& bodyAngles, const unsigned int dt);
  bool finished();

private:
  Interpolator comInterpolator_;
  Interpolator torsoAngleInterpolator_;
  float progress_;
  KinematicMatrix left2right_;
};

class Lift : public KickPhase
{
public:
  Lift(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration);
  void reset(const std::vector<float>& previousAngles);
  void getAngles(std::vector<float>& bodyAngles, const unsigned int dt);
  bool finished();

private:
  Interpolator liftInterpolator_;
  Interpolator anklePitchInterpolator_;
  KinematicMatrix com2right_;
};

/// commence single support by lifting leg, then swing it to kick
class Swing : public KickPhase
{
public:
  Swing(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration);
  void reset(const std::vector<float>& previousAngles);
  void getAngles(std::vector<float>& bodyAngles, const unsigned int dt);
  bool finished();

private:
  const Parameter<float> canonicalSystemFinalValue_;
  const Parameter<std::vector<float>> weightings_;
  DynamicMovementPrimitive dynamicMovementPrimitive_;
  Interpolator anklePitchInterpolator_;
  KinematicMatrix com2right_;
};

/// retract the leg
class Retract : public KickPhase
{
public:
  Retract(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration);
  void reset(const std::vector<float>& previousAngles);
  void getAngles(std::vector<float>& bodyAngles, const unsigned int dt);
  bool finished();

private:
  const Parameter<Vector3f> retractPosition_;
  Interpolator retractInterpolator_;
  Interpolator anklePitchInterpolator_;
  KinematicMatrix com2right_;
};

/// extend the leg to establish double support again while simultaneously shifting the CoM back to a position between the two feet
class ExtendAndCenter : public KickPhase
{
public:
  ExtendAndCenter(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration);
  void reset();
  void getAngles(std::vector<float>& bodyAngles, const unsigned int dt);
  bool finished();

private:
  const Parameter<Vector3f> retractPosition_;
  const Parameter<Vector3f> extendPosition_;
  Interpolator extendInterpolator_;
  Interpolator centerInterpolator_;
  Interpolator torsoAngleInterpolator_;
};

/// wait briefly after the kick before exiting safely
class Wait : public KickPhase
{
public:
  Wait(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration);
  void reset(const std::vector<float>& previousAngles);
  void getAngles(std::vector<float>& bodyAngles, const unsigned int dt);
  bool finished();

private:
  Interpolator interpolator_;
};

/// catch the robot by interpolating to the ready pose
class CatchFallen : public KickPhase
{
public:
  CatchFallen(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration);
  void reset(const std::vector<float>& previousAngles);
  void getAngles(std::vector<float>& bodyAngles, const unsigned int dt);
  bool finished();

private:
  Interpolator interpolator_;
};
