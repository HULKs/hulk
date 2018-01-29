#include <Modules/Debug/Debug.h>
#include <Tools/Kinematics/InverseKinematics.h>

#include "Pendulum.hpp"
#include "print.h"

/** Constructor **/
Pendulum::Pendulum(const ModuleBase& module, const MotionPlannerOutput& motionPlannerOutput, const IMUSensorData& imuSensorData, Debug& debug)
  : origin_(module, "origin", [this] { s_->rby = origin_().y() * -s_->support; })
  , periodDuration_(module, "periodDuration", [this] { s_->x0by = s_->support * origin_().y() / cosh(0.5f * periodDuration_() * s_->k); })
  , height_(module, "height", [this] { s_->k = sqrt(gravity_() / height_()); })
  , gravity_(module, "gravity", [this] { s_->k = sqrt(gravity_() / height_()); })
  , speedCorrection_(module, "speedCorrection", [] {})
  , stepDeadTime_(module, "stepDeadTime", [] {})
  , originLimit_(module, "originLimit", [] {})
  , swingLimitY_(module, "swingLimitY", [] {})
  , stepLimitX_(module, "stepLimitX", [] {})
  , maxAngleVelY_(module, "maxAngleVelY", [] {})
  , setDynamicSteps_(module, "setDynamicSteps", [] {})
  , imuSensorData_(imuSensorData)
  , debug_(debug)
  , footController_(module, imuSensorData)
  , zmpLimit_(originLimit_().x())
  , maxImuError_(0)
  , maxLastImuError_(0)
  , stepDampX_(1)
  , dynamicStepAccumulator_(0.f)
  , isStepLimDone_(true)
  , stepPlanner_(module, motionPlannerOutput)
{
  s_ = new PendulumStates();
  s_->k = sqrt(gravity_() / height_());
  reset();
}

Pendulum::~Pendulum()
{
  delete s_;
}

/** updateParameters **/
void Pendulum::updateParameters(const ComOffset& offset, const bool& fromStand, const bool startStepping, const InWalkKickType requestedKickType)
{
  if (requestedKickType != InWalkKickType::NONE && requestedKickType_ == InWalkKickType::NONE)
  {
    requestedKickType_ = requestedKickType;
  }

  if (startStepping)
  {
    s_->desiredStep = Pose();
  }

  if (s_->support == SF_LEFT_SUPPORT)
  {
    /// Store correction parameters
    s_->corrX = offset.offsetFromLeftX;
    s_->corrY = offset.offsetFromLeftY;
  }
  else
  {
    s_->corrX = offset.offsetFromRightX;
    s_->corrY = offset.offsetFromRightY;
  }

  /// estimate the current com position and velocity in y-direction
  s_->com.y() = s_->ry + s_->corrY[0] + s_->x0y * cosh(s_->k * s_->time);
  s_->vel.y() = s_->corrY[1] + s_->k * s_->x0y * sinh(s_->k * s_->time);

  /// calculate origin of pendulum model in y-direction
  s_->ry = s_->com.y() - s_->x0y * sqrt(pow(s_->vel.y() / s_->x0y / s_->k, 2) + 1);

  /// limit origin position
  float ryDiff = s_->ry - origin_().y() * s_->support;
  if (fabs(ryDiff) > originLimit_().y())
  {
    s_->ry = origin_().y() * s_->support + SIGN(ryDiff) * originLimit_().y();

    /// only if the origin of the pendulum had to be limited, the
    /// pendulum is allowed to swing more or less than desired
    float arg = (pow(s_->com.y() - s_->ry, 2) - pow(s_->vel.y() / s_->k, 2));

    /// if the pendulum is going to swing more then allowd -> limit
    if (arg < 0 || sqrt(arg) < swingLimitY_())
    {
      s_->x0y = swingLimitY_() * -s_->support;
    }
    /// compute new maximal swing width
    else
    {
      s_->x0y = sqrt(arg) * -s_->support;
    }
  }

  /// get time of the pendulum
  s_->time = 1 / s_->k * asinh(s_->vel.y() / (s_->k * (s_->x0y)));

  /// ************************ ///
  /// * SUPPORT CHANGE TIMES * ///
  /// ************************ ///

  float a = -s_->ry + s_->step.position.y() + s_->rby;

  s_->tbb = -1 / s_->k * acosh((pow(s_->x0y, 2) - pow(a, 2) - pow(s_->x0by, 2)) / (2 * a * s_->x0by));

  /// tB is NAN if the boundary conditions can not be met.
  /// A support change is required at time zero
  if (std::isnan(s_->tbb))
    s_->tbb = -0.00001f; /// 0 not possible

  s_->te = 1 / s_->k * asinh((s_->x0by * sinh(s_->k * s_->tbb) / s_->x0y));

  /// check if initial time has to be set
  if (fromStand)
    s_->tb = s_->time;

  /// check for support change
  if (s_->time >= s_->te)
  {
    switchSupport(offset);
    return;
  }

  /// ************************ ///
  ///    UPDATE X-DIRECTION    ///
  /// ************************ ///

  /// ************************ ///
  ///   NO STEP IS DESIRED     ///
  /// ************************ ///

  /// cubic spline
  float A = 2 * s_->p0 - 2 * s_->p1 + s_->m0 + s_->m1;
  float B = -3 * s_->p0 + 3 * s_->p1 - 2 * s_->m0 - s_->m1;

  /// scale t to a range between 0 and 1
  float t = (s_->time - s_->tb) / (s_->te - s_->tb);

  s_->com.x() = A * pow(t, 3) + B * pow(t, 2) + s_->m0 * t + s_->p0 + s_->corrX[0];
  s_->vel.x() = A * 3 * pow(t, 2) + B * 2 * t + s_->m0 + s_->corrX[1];

  /// don't calculate new parameters when end of phase will be reached soon
  if (t < 0.8)
  {
    /// Set step size to desired step size
    s_->step.position.x() = s_->desiredStep.position.x();

    /// target position is step/2
    s_->p1 = s_->step.position.x() / 2;

    /// target velocity to satisfy boundary conditions for next phase.
    /// From motion equations of 3D Linear inverted Pendulum (like for
    /// y-direction)
    /// A scale factor is multiplicated to shape a smooth spline curve.
    s_->m1 = (-s_->step.position.x() / 2 * s_->k * cosh(s_->k * s_->tbb)) / sinh(s_->k * s_->tbb) * speedCorrection_();

    /// Calculate new spline parameters m0 and p0 to satisfy the conditions:
    /// x(t)   = comX
    /// x'(t)  = velX
    /// x(te)  = step/2
    /// x'(te) = m1 (like calculated above)
    float z = s_->com.x() - (-2 * s_->p1 + s_->m1) * pow(t, 3) - (3 * s_->p1 - s_->m1) * pow(t, 2);
    float y = s_->vel.x() - 3 * pow(t, 2) * (-2 * s_->p1 + s_->m1) - 2 * t * (3 * s_->p1 - s_->m1);

    s_->m0 = (y - z * (6 * pow(t, 2) - 6 * t) / (2 * pow(t, 3) - 3 * pow(t, 2) + 1)) /
             (3 * pow(t, 2) - 4 * t + 1 - (pow(t, 3) - 2 * pow(t, 2) + t) / (2 * pow(t, 3) - 3 * pow(t, 2) + 1) * (6 * pow(t, 2) - 6 * t));
    s_->p0 = (z - s_->m0 * (pow(t, 3) - 2 * pow(t, 2) + t)) / (2 * pow(t, 3) - 3 * pow(t, 2) + 1);

    /// calculation of zmp for current time step and for end of phase
    float accX = (12 * t - 6) * s_->p0 + (-12 * t + 6) * s_->p1 + (6 * t - 4) * s_->m0 + (6 * t - 2) * s_->m1;
    s_->zmp = s_->com.x() - accX * height_() / gravity_();

    // Comparison of old an new Windows:
    if (setDynamicSteps_())
    {
      // Dynamically limiting the zmp
      maxImuVel_ = std::max(maxImuVel_, std::fabs(imuSensorData_.angle.y()));
      float imuVel = std::max(maxImuVel_, maxLastImuVel_);

      if (imuVel > maxAngleVelY_() && !isStepLimDone_)
      {
        zmpLimit_ *= stepDampX_;
        isStepLimDone_ = true;
      }
    }
    else
    {
      zmpLimit_ = originLimit_().x();
    }

    /// check limitations for zmp
    if (fabsf(s_->zmp) > zmpLimit_)
    {
      /// limit the zmp
      s_->zmp = zmpLimit_ * SIGN(s_->zmp);

      /// Where can the com get when holding zmp?
      /// solving x'' = (x-p) * g/h, where x = com position and p = zmp.
      /// The solution is x(t) = c1 * exp(k*t) + c2 * exp(-k*t) + p for some c1, c2.
      /// Here, c1 = (x(t) - p(t) + x'(t) / k) / 2 and c2 = (x(t) - p(t) - x'(t) / k) / 2 where t = current time.

      /// end position
      s_->p1 = s_->zmp + (exp(s_->k * (s_->te - s_->time)) * (s_->com.x() - s_->zmp + s_->vel.x() / s_->k) +
                          exp(s_->k * (s_->time - s_->te)) * (s_->com.x() - s_->zmp - s_->vel.x() / s_->k)) /
                             2;

      /// end velocity
      s_->m1 = (exp(s_->k * (s_->te - s_->time)) * (s_->k * (s_->com.x() - s_->zmp) + s_->vel.x()) -
                exp(s_->k * (s_->time - s_->te)) * (s_->k * (s_->com.x() - s_->zmp) - s_->vel.x())) /
               2;

      /// from current measurement
      s_->p0 = -(s_->com.x() - 3 * s_->com.x() * t + 3 * s_->p1 * t * t - s_->p1 * t * t * t - t * t * s_->m1 + t * t * t * s_->m1) /
               ((t - 1) * (t * t - 2 * t + 1));

      s_->m0 = (6 * s_->p1 * t - 6 * s_->com.x() * t - 2 * t * s_->m1 + t * t * s_->m1 + t * t * t * s_->m1) / ((t - 1) * (t * t - 2 * t + 1));

      /// The step size is calculated from the end position of the com and
      /// the condition that the inverted pendulum model shall reach the origin
      /// of the next phase at t = 0
      s_->step.position.x() = s_->p1 - s_->m1 / s_->k * tanh(s_->k * s_->tbb);

      /// limit step size if neccessary
      if (fabsf(s_->step.position.x()) > stepLimitX_())
        s_->step.position.x() = stepLimitX_() * SIGN(s_->step.position.x());
    }
  }

  // Debug output:

  debug_.update("Motion.Pendulum.abort", s_->abort);
  debug_.update("Motion.Pendulum.com", s_->com);
  debug_.update("Motion.Pendulum.corrX", s_->corrX);
  debug_.update("Motion.Pendulum.corrY", s_->corrY);
  debug_.update("Motion.Pendulum.desiredStep", s_->desiredStep);
  debug_.update("Motion.Pendulum.lastStep", s_->lastStep);
  debug_.update("Motion.Pendulum.m0", s_->m0);
  debug_.update("Motion.Pendulum.m1", s_->m1);
  debug_.update("Motion.Pendulum.p0", s_->p0);
  debug_.update("Motion.Pendulum.p1", s_->p1);
  debug_.update("Motion.Pendulum.rby", s_->rby);
  debug_.update("Motion.Pendulum.ry", s_->ry);
  debug_.update("Motion.Pendulum.step", s_->step);
  debug_.update("Motion.Pendulum.tb", s_->tb);
  debug_.update("Motion.Pendulum.tbb", s_->tbb);
  debug_.update("Motion.Pendulum.te", s_->te);
  debug_.update("Motion.Pendulum.time", s_->time);
  debug_.update("Motion.Pendulum.torsoMatrixChange", s_->torsoMatrixChange);
  debug_.update("Motion.Pendulum.vel", s_->vel);
  debug_.update("Motion.Pendulum.x0by", s_->x0by);
  debug_.update("Motion.Pendulum.x0y", s_->x0y);
  debug_.update("Motion.Pendulum.zmp", s_->zmp);
}

/** computeExpectedCom **/
void Pendulum::computeExpectedCom(ComPosition& comPos, float& angleL, float& angleR)
{
  /// predict the com position relative to the torso
  s_->com.y() = s_->ry + s_->x0y * cosh(s_->k * s_->time);

  /// scale time to a range between 0 and 1
  float t = (s_->time - s_->tb) / (s_->te - s_->tb);

  /// cubic spline equation for the com position relative to the x-origin
  s_->com.x() = (2 * s_->p0 - 2 * s_->p1 + s_->m0 + s_->m1) * pow(t, 3) + (-3 * s_->p0 + 3 * s_->p1 - 2 * s_->m0 - s_->m1) * pow(t, 2) + s_->m0 * t + s_->p0;

  /// To get the com relative to the feet, the origin is added.
  comPos.fromLeft = Vector3f(s_->com.x() + origin_().x(), s_->com.y() - origin_().y(), height_());
  comPos.fromRight = Vector3f(s_->com.x() + origin_().x(), s_->com.y() + origin_().y(), height_());

  Vector3f stp = Vector3f::Zero();
  float angle;

  /// calculation of the step offsets for the swinging foot
  getStep(s_->time, stp, angle);

  /// add the step offsets to the swinging foot
  if (s_->support == SF_LEFT_SUPPORT)
  {
    comPos.fromRight -= stp;
    angleL = angle / 2;
    angleR = -angle / 2;
  }
  else
  {
    comPos.fromLeft -= stp;
    angleL = -angle / 2;
    angleR = angle / 2;
  }
}

void Pendulum::computeStandCom(ComPosition& comPos)
{
  comPos.fromLeft = Vector3f(origin_().x(), -origin_().y(), height_());
  comPos.fromRight = Vector3f(origin_().x(), origin_().y(), height_());
}

/** Pendulum::step **/
void Pendulum::getStep(const float& time, Vector3f& step, float& angle)
{
  float progress = (time - s_->tb) / (s_->te - s_->tb);
  FootPose3D currentFootPose;
  Step2D targetFootPose{s_->step.position, s_->step.orientation};
  Step2D lastFootPose{s_->lastStep.position, s_->lastStep.orientation};

  if (((s_->support == SF_LEFT_SUPPORT && (requestedKickType_ == InWalkKickType::RIGHT_GENTLE || requestedKickType_ == InWalkKickType::RIGHT_STRONG)) ||
       (s_->support == SF_RIGHT_SUPPORT && (requestedKickType_ == InWalkKickType::LEFT_GENTLE || requestedKickType_ == InWalkKickType::LEFT_STRONG))) &&
      progress > 0.1 && progress < 0.2)

  {
    currentPhaseKickType_ = requestedKickType_;
  }


  /// FootControler manipulates currentFootPose
  footController_.getStep(progress, currentFootPose, targetFootPose, lastFootPose, currentPhaseKickType_, maxImuError_, maxLastImuError_,
                          dynamicStepAccumulator_);


  /// For now copy the result of the foot controller into the former representation
  step = currentFootPose.position;
  angle = currentFootPose.orientation;
}

/** updateRequest **/
void Pendulum::updateRequest(WalkingType type)
{
  s_->request = type;
  if (s_->request == STEPPING)
  {
    s_->abort = false;
    s_->stopNextStep = false;
  }
}

/** reset **/
void Pendulum::reset()
{
  s_->support = SF_LEFT_SUPPORT;
  s_->x0y = -s_->support * origin_().y() / cosh(0.5f * periodDuration_() * s_->k);
  s_->x0by = -s_->x0y;
  s_->ry = origin_().y() * s_->support;
  s_->rby = -s_->ry;
  s_->step = Pose();
  s_->lastStep = Pose();
  s_->desiredStep = Pose();
  s_->time = 0;
  s_->tb = 0;
  s_->te = 0;
  s_->tbb = 0;
  s_->p0 = 0;
  s_->p1 = 0;
  s_->m0 = 0;
  s_->m1 = 0;
  s_->request = STAND;
  s_->abort = true;
  s_->stopNextStep = false;

  /// InWalkKick
  requestedKickType_ = InWalkKickType::NONE;
  currentPhaseKickType_ = InWalkKickType::NONE;

  /// Resetting dynamic step stuff, since the information from the last step isn't valid anymore.
  maxImuError_ = 0;
  maxLastImuError_ = 0;
  isStepLimDone_ = false;
  zmpLimit_ = originLimit_().x();
  stepDampX_ = 1;
  dynamicStepAccumulator_ = imuSensorData_.angle.y();
}

/** switchSupport **/
void Pendulum::switchSupport(const ComOffset& offset)
{
  /// Reset plannedKickType if we actually performed it:
  if (currentPhaseKickType_ == requestedKickType_)
  {
    requestedKickType_ = InWalkKickType::NONE;
  }
  currentPhaseKickType_ = InWalkKickType::NONE;

  /**
   * Comparing the IMU vel and error of the last two steps
   * as a criteria of stability
   */
  if (maxImuError_ > maxLastImuError_ && maxImuVel_ > maxAngleVelY_())
  {
    stepDampX_ *= 0.9;
  }
  else if (maxImuVel_ < maxAngleVelY_())
  {
    stepDampX_ = 1;
    zmpLimit_ = originLimit_().x();
  }
  /// Reallow step limitaiton for next step.
  isStepLimDone_ = false;

  /// Shifting the IMU measurements of previous steps:
  maxLastImuVel_ = maxImuVel_;
  maxImuVel_ = 0;
  maxLastImuError_ = maxImuError_;
  maxImuError_ = 0;

  /// changing parameters when support is changed
  s_->support = s_->support == SF_LEFT_SUPPORT ? SF_RIGHT_SUPPORT : SF_LEFT_SUPPORT;

  s_->torsoMatrixChange = s_->step;

  /// stop if this was the last walking phase after a stop command arrived
  if (s_->stopNextStep)
  {
    s_->abort = true;
    return;
  }

  /// store information about current step and adjust for pathplanner,
  /// because the autocollision avoidance when going sidewards also avoids
  /// increasing the step size if there are always 0 steps inbetween.
  Pose currentStep = s_->step;
  if ((s_->support == SF_LEFT_SUPPORT && s_->lastStep.position.y() > 0) || (s_->support == SF_RIGHT_SUPPORT && s_->lastStep.position.y() < 0))
    currentStep.position.y() = -s_->lastStep.position.y();

  s_->lastStep = s_->step.inverse();

  /// determine next step and handle stopping
  if (s_->request == STAND)
  {
    s_->stopNextStep = true;
    s_->step = Pose();
  }
  else
  {
    s_->step = stepPlanner_.nextStep(currentStep, s_->support, periodDuration_());
    if (s_->request == PREPARING_STAND && s_->step.position.norm() < 0.01f && std::abs(s_->step.orientation) < 0.01 && s_->lastStep.position.norm() < 0.01f &&
        std::abs(s_->lastStep.orientation) < 0.01)
    {
      s_->abort = true;
      return;
    }
  }
  s_->desiredStep = s_->step;

  /// parameters for next phase
  s_->ry = s_->rby;
  s_->x0y = s_->x0by;
  s_->rby = -s_->rby;
  s_->x0by = -s_->x0by;
  // When s_->time has been greater than s_->te, this time has already elapsed in the new phase.
  // That gives s_->time := s_->tbb + s_->time - s_->te or in short:
  s_->time += s_->tbb - s_->te;
  s_->p0 = s_->p1 + s_->lastStep.position.x();
  s_->m0 = s_->m1;

  //  outstream.open(pathprefix +  "steps_cpp.csv", ios_base::app);
  //  outstream << step.x() << ";";
  //  outstream << step.y() << ";";
  //  outstream << step.z() << std::endl;
  //  outstream.close();

  /// update the pendulum parameters
  updateParameters(offset, true, false, requestedKickType_);
}

/** timeStep **/
void Pendulum::timeStep()
{
  /// adds one sample to the time
  s_->time += TIME_STEP;
}

Pose Pendulum::getTorsoMatrixChange()
{
  return s_->torsoMatrixChange;
}

float Pendulum::getTimePercentage()
{
  const float phaseDuration = s_->te - s_->tb;
  if (phaseDuration < 0.0001f)
  {
    return 0;
  }
  else
  {
    return (s_->time - s_->tb) / phaseDuration;
  }
}

bool Pendulum::isAborted()
{
  return s_->abort;
}

supportFoot Pendulum::getSupport() const
{
  return s_->support;
}
