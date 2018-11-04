#include <functional>

// tuhhSDK includes
#include <Modules/Poses.h>
#include <Tools/Kinematics/Com.h>
#include <Tools/Kinematics/ForwardKinematics.h>
#include <Tools/Kinematics/InverseKinematics.h>
#include <Tools/Time.hpp>

// Motion includes
#include "Poser/Poser.hpp"
#include "WalkingEngine.hpp"

#include "print.hpp"

/** Constructor **/
WalkingEngine::WalkingEngine(const ModuleManagerInterface& manager)
  : Module(manager)
  , positionOffsetLeft_(*this, "positionOffsetLeft", [] {})
  , positionOffsetRight_(*this, "positionOffsetRight", [] {})
  , torsoAngle_(*this, "torsoAngle", [this] { torsoAngle_() *= TO_RAD; })
  , walkStiffness_(*this, "walkStiffness", [] {})
  , // TODO: Add dependency on  battery level
  hipCorrectionY_(*this, "hipCorrectionY", [] {})
  , angleOffsetLeft_(*this, "angleOffsetLeft", [this] { angleOffsetLeft_() *= TO_RAD; })
  , angleOffsetRight_(*this, "angleOffsetRight", [this] { angleOffsetRight_() *= TO_RAD; })
  , linearVel_(*this, "linearVel", [] {})
  , periodDuration_(*this, "periodDuration", [] {})
  , rotationAngleLimit_(*this, "rotationAngleLimit", [this] {rotationAngleLimit_() *= TO_RAD; })
  , kickInWalk_(*this, "kickInWalk", [] {})
  , kalmanQ_(*this, "kalmanQ")
  , kalmanR_(*this, "kalmanR")
  , lowPassAlphaAnkle_(*this, "lowPassAlphaAnkle", [] {})
  , motionActivation_(*this)
  , motionPlannerOutput_(*this)
  , bodyPose_(*this)
  , imuSensorData_(*this)
  , jointSensorData_(*this)
  , robotKinematics_(*this)
  , walkingEngineWalkOutput_(*this)
  , walkingEngineStandOutput_(*this)
  , pendulum_(*this, *motionPlannerOutput_, *imuSensorData_, debug())
  , ankleAccumulator_(0.f)
  , lastComID_(0)
  , currentWalkType_(STAND)
  , nextWalkType_(STAND)
  , walkAngles_(JOINTS::JOINTS_MAX)
  , standAngles_(JOINTS::JOINTS_MAX)
  , fromStand_(false)
  , startStepping_(true)
  , active_(false)
  , countPose_(false)
  , poseCountFinish_(0)
  , lastSupport_(SF_DOUBLE_SUPPORT)
  , projectedTorsoPosition_(0, 0)
  , lastShift_(0, 0)
{
  print("WalkingEngine: Initializing WalkingEngine", LogLevel::INFO);

  // These parameters are given in degrees so they need to be converted. There needs to be a better way for this.
  torsoAngle_() *= TO_RAD;
  angleOffsetLeft_() *= TO_RAD;
  angleOffsetRight_() *= TO_RAD;
  rotationAngleLimit_() *= TO_RAD;
  Matrix2f kalmanInitA;
  kalmanInitA << 1, TIME_STEP, 0, 1;
  kalmanY_ = KalmanFilter(kalmanInitA,                           // A
                          Vector2f(0, 0),                        // b
                          Vector2f(1, 0),                        // c
                          Vector2f::Zero(),                      // x
                          Matrix2f::Identity(),                  // P
                          Matrix2f::Identity() * kalmanQ_().y(), // Q
                          kalmanR_().y()                         // R
  );
  kalmanX_ = KalmanFilter(kalmanInitA,                           // A
                          Vector2f(0, 0),                        // b
                          Vector2f(1, 0),                        // c
                          Vector2f::Zero(),                      // x
                          Matrix2f::Identity(),                  // P
                          Matrix2f::Identity() * kalmanQ_().x(), // Q
                          kalmanR_().x()                         // R
  );
  generateStandAngles();
}

void WalkingEngine::cycle()
{
  // Filter gyro to use it in ankle controller:
  ankleAccumulator_ = lowPassAlphaAnkle_() * imuSensorData_->gyroscope.y() + (1 - lowPassAlphaAnkle_()) * ankleAccumulator_;

  // the controll output is applied to a separate set of angle since the standAngles_ are not reseted every cycle
  std::vector<float> controlledStandAngles = standAngles_;
  applyAnkleController(controlledStandAngles);

  /// For default case always use the stand angles, just to make sure vectors are of correct sice:
  walkingEngineStandOutput_->angles = controlledStandAngles;
  walkingEngineStandOutput_->stiffnesses = std::vector<float>(walkingEngineStandOutput_->angles.size(), walkStiffness_());

  if (motionActivation_->activations[static_cast<unsigned int>(MotionPlannerOutput::BodyMotion::WALK)] == 1 &&
      motionPlannerOutput_->bodyMotion == MotionPlannerOutput::BodyMotion::WALK)
  {
    start();
  }
  else if (motionActivation_->activations[static_cast<unsigned int>(MotionPlannerOutput::BodyMotion::WALK)] > 0 ||
           motionActivation_->activations[static_cast<unsigned int>(MotionPlannerOutput::BodyMotion::STAND)] > 0)
  {
    stop(motionPlannerOutput_->walkStopData.gracefully);
  }
  else
  {
    stop(false);
  }

  if (countPose_)
  {
    countPose();
  }

  if (active_ && bodyPose_->fallen)
  {
    disconnect();
  }

  // It can be assumed that the robot is upright here since a few lines above we would have been disconnected.
  if (active_)
  {
    // print("WalkingEngine is active!", LogLevel::INFO);
    /// Update of the Pendulum model from measurement
    updateStates();

    /// Odometry:
    reportOdometry();

    walkingEngineWalkOutput_->angles = walkAngles_;
    walkingEngineWalkOutput_->stiffnesses = std::vector<float>(walkAngles_.size(), walkStiffness_());
  }
  else
  {
    walkingEngineWalkOutput_->angles = controlledStandAngles;
    walkingEngineWalkOutput_->stiffnesses = std::vector<float>(walkingEngineWalkOutput_->angles.size(), walkStiffness_());
    walkingEngineStandOutput_->angles = controlledStandAngles;
    walkingEngineStandOutput_->stiffnesses = std::vector<float>(walkingEngineStandOutput_->angles.size(), walkStiffness_());
  }
  walkingEngineWalkOutput_->safeExit = pendulum_.isAborted();

  // publish max velocities
  const float rotationalVel = rotationAngleLimit_() / periodDuration_() * 0.5f;
  walkingEngineWalkOutput_->maxVelocityComponents =
      Pose(linearVel_(), 0.2f * linearVel_(), rotationalVel);
  // this relation was found empirically, don't ask.
  walkingEngineWalkOutput_->walkAroundBallVelocity = rotationalVel * 2.f / 3.f;
}

void WalkingEngine::pushOdometryUpdate(const Vector2f& torsoShift)
{
  walkingEngineWalkOutput_->stepOffset.position += torsoShift;
}


/** updateStates **/
void WalkingEngine::updateStates()
{
  /// update Request
  updateRequest();

  /// measurement of com relative to both feet
  measure();

  /// if necessary, initialize lastComs so updateError has valid data there
  if (fromStand_)
  {
    for (int i = 0; i < 4; i++)
    {
      lastComs_[i].fromLeft = measuredCom_.fromLeft;
      lastComs_[i].fromRight = measuredCom_.fromRight;
    }
  }

  /// predict error using kalman filters
  updateError();

  /// update the offset to be added to the model predictions
  updateOffset();


  if (kickInWalk_())
  {
    pendulum_.updateParameters(comOffset_, fromStand_, startStepping_, InWalkKickType::RIGHT_STRONG);
    kickInWalk_() = false; /// Reset in fashion of a "single debug response request"
  }
  else
  {
    /// update pendulum and spline parameters
    pendulum_.updateParameters(comOffset_, fromStand_, startStepping_, motionPlannerOutput_->walkData.inWalkKickType);
  }

  startStepping_ = false;

  if (!pendulum_.isAborted())
  {
    fromStand_ = false;

    /// update time parameter
    pendulum_.timeStep();

    /// update the expected position of the com
    pendulum_.computeExpectedCom(comCommand_, angleL_, angleR_);

    /// store the last command
    lastComs_[lastComID_] = comCommand_;
    lastComID_ = (lastComID_ + 1) % 4;

    /// compute joint angles for comCommand
    computeLegAngles2Com(walkAngles_, comCommand_, angleL_, angleR_);
    applyAnkleController(walkAngles_);
  }
  else
  {
    print("WalkingEngine: Pendulum aborted. Going to disconnect.", LogLevel::INFO);
    /// disconnect the motion cycle
    disconnect();
    // enable Countdown.
    poseCountFinish_ = 500;
    countPose_ = true;
  }
}

/** measure **/
void WalkingEngine::measure()
{
  /// get com relative to torso - it does not have any rotation component
  KinematicMatrix com = KinematicMatrix(robotKinematics_->com);

  const Vector3f& angle = imuSensorData_->angle;

  KinematicMatrix imu = KinematicMatrix::rotY(angle.y()) * KinematicMatrix::rotX(angle.x());

  measuredCom_.fromLeft = imu * (com.posV - robotKinematics_->matrices[JOINTS::L_FOOT].posV) / 1000;
  measuredCom_.fromRight = imu * (com.posV - robotKinematics_->matrices[JOINTS::R_FOOT].posV) / 1000;
}

/** start **/
void WalkingEngine::start()
{
  nextWalkType_ = STEPPING;
  active_ = true;
}

/** stop **/
void WalkingEngine::stop(bool gracefully)
{
  if (currentWalkType_ == STAND)
  {
    print("WalkingEngine: I have already stopped walking", LogLevel::INFO);
  }
  else
  {
    if (gracefully)
    {
      print("WalkingEngine: I will stop walking gracefully", LogLevel::INFO);
      nextWalkType_ = PREPARING_STAND;
    }
    else
    {
      print("WalkingEngine: I will stop walking as soon as possible", LogLevel::INFO);
      nextWalkType_ = STAND;
    }
  }
}

/** disconnect **/
void WalkingEngine::disconnect()
{
  currentWalkType_ = nextWalkType_ = STAND;
  active_ = false;
  pendulum_.reset();
}

/** updateError **/
void WalkingEngine::updateError()
{
  /// The current measurement is compared to the command that was sent four cycles ago.
  /// This is due to a delay in the hardware.
  errorCom_.fromLeft = measuredCom_.fromLeft - lastComs_[lastComID_].fromLeft;
  errorCom_.fromRight = measuredCom_.fromRight - lastComs_[lastComID_].fromRight;
}

/** updateOffset **/
void WalkingEngine::updateOffset()
{
  Vector2f kalmanGain = kalmanY_.predictGain();

  /// y-direction
  comOffset_.offsetFromLeftY = kalmanGain * errorCom_.fromLeft.y();
  comOffset_.offsetFromRightY = kalmanGain * errorCom_.fromRight.y();

  /// x-direction
  kalmanGain = kalmanX_.predictGain();
  comOffset_.offsetFromLeftX = kalmanGain * errorCom_.fromLeft.x();
  comOffset_.offsetFromRightX = kalmanGain * errorCom_.fromRight.x();
}

/** updateRequest **/
void WalkingEngine::updateRequest()
{
  if (currentWalkType_ == STAND && nextWalkType_ == STEPPING)
  {
    projectedTorsoPosition_ = getProjectedTorsoPostion();
    ankleAccumulator_ = imuSensorData_->gyroscope.y();
    pendulum_.reset();
    pendulum_.updateRequest(STEPPING);
    currentWalkType_ = STEPPING;
    lastComID_ = 0;
    fromStand_ = true;
    startStepping_ = true;
  }
  else if (currentWalkType_ != STAND && nextWalkType_ == STAND)
  {
    generateStandAngles();
    pendulum_.updateRequest(STAND);
    currentWalkType_ = STAND;
  }
  else if (currentWalkType_ == STEPPING && nextWalkType_ == PREPARING_STAND)
  {
    pendulum_.updateRequest(PREPARING_STAND);
    currentWalkType_ = PREPARING_STAND;
  }
  else if (currentWalkType_ == PREPARING_STAND && nextWalkType_ == STEPPING)
  {
    pendulum_.updateRequest(STEPPING);
    currentWalkType_ = STEPPING;
  }
  else if (currentWalkType_ != nextWalkType_)
  {
    print("WalkingEngine: Illegal walk type transition. Staying at current walk type.", LogLevel::WARNING);
    nextWalkType_ = currentWalkType_;
  }
}

/** computeLegAngles2Com **/
void WalkingEngine::computeLegAngles2Com(std::vector<float>& bodyAngles, const ComPosition& comCommand, const float angleL, const float angleR,
                                         bool computeForStand)
{
  KinematicMatrix com2left, com2right, left2com, right2com;

  /// set com positions to uncalibrated values that were calculated by the pendulum
  com2left.posV = comCommand.fromLeft;
  com2left.rotM = AngleAxisf(angleL, Vector3f::UnitZ());

  com2right.posV = comCommand.fromRight;
  com2right.rotM = AngleAxisf(angleR, Vector3f::UnitZ());

  /// add calibration and convert to millimeters
  com2left.posV += positionOffsetLeft_();
  com2left.posV *= 1000;
  com2left.rotM = com2left.rotM * AngleAxisf(angleOffsetLeft_(), Vector3f::UnitZ()) * AngleAxisf(torsoAngle_(), Vector3f::UnitY());

  com2right.posV += positionOffsetRight_();
  com2right.posV *= 1000;
  com2right.rotM = com2right.rotM * AngleAxisf(angleOffsetRight_(), Vector3f::UnitZ()) * AngleAxisf(torsoAngle_(), Vector3f::UnitY());

  /// invert to get the feet to com
  left2com = com2left.invert();
  right2com = com2right.invert();

  /// Now the joint angles for the legs are computed in order to
  /// move the center of mass to a desired position relative to the feet.
  /// Since the calculated angles will change the location of the center of
  /// mass, an iterative method is used, which takes the actual position of the
  /// center of mass as starting point.

  /// starting point for center of mass and current joint angles
  KinematicMatrix com2torso;
  std::vector<float> lLegAngles, rLegAngles;

  if (computeForStand)
  {
    bodyAngles = Poses::getPose(Poses::PENALIZED);
    com2torso = KinematicMatrix(Com::getCom(bodyAngles));
  }
  else
  {
    bodyAngles = jointSensorData_->getBodyAngles();
    com2torso = KinematicMatrix(robotKinematics_->com);
  }

  /// difference between actual and desired center of mass
  Vector3f error;

  for (int i = 0; i < 5; i++)
  {
    /// compute legAngles relativ com2torso, which will be modified during
    /// the iterations
    if (pendulum_.getSupport() == SF_LEFT_SUPPORT)
    {
      lLegAngles = InverseKinematics::getLLegAngles(com2torso * left2com);
      rLegAngles = InverseKinematics::getFixedRLegAngles(com2torso * right2com, lLegAngles[0]);
    }
    else
    {
      rLegAngles = InverseKinematics::getRLegAngles(com2torso * right2com);
      lLegAngles = InverseKinematics::getFixedLLegAngles(com2torso * left2com, rLegAngles[0]);
    }

    /// put computed leg angles in joint angle vector for whole body
    for (int j = 0; j < JOINTS_L_LEG::L_LEG_MAX; j++)
    {
      bodyAngles[JOINTS::L_HIP_YAW_PITCH + j] = lLegAngles[j];
      bodyAngles[JOINTS::R_HIP_YAW_PITCH + j] = rLegAngles[j];
    }

    /// Where would the com be after setting these angles?
    KinematicMatrix com2torso_l = KinematicMatrix(Com::getCom(bodyAngles));

    /// compute resulting position of the feet when applying the calculated
    /// angles
    KinematicMatrix foot2torso;
    if (pendulum_.getSupport() == SF_LEFT_SUPPORT)
    {
      foot2torso = ForwardKinematics::getLFoot(lLegAngles);
    }
    else
    {
      foot2torso = ForwardKinematics::getRFoot(rLegAngles);
    }

    /// calculate the resulting position of the center of mass relative to the
    /// support foot
    KinematicMatrix com2foot = foot2torso.invert() * com2torso_l;

    /// calculate the error between the desired position of the center of mass
    /// and the currently calculated resulting position
    if (pendulum_.getSupport() == SF_LEFT_SUPPORT)
    {
      error = com2foot.posV - com2left.posV;
    }
    else
    {
      error = com2foot.posV - com2right.posV;
    }

    /// update the starting point for the iteration
    com2torso.posV.x() += error.x();
    com2torso.posV.y() += error.y();
  }

  /// adding a sine wave to the hip_roll joint significantly supresses the
  /// bending of the hip when lifting a foot
  if (!computeForStand)
  {
    if (pendulum_.getSupport() == SF_LEFT_SUPPORT)
    {
      lLegAngles[JOINTS_L_LEG::L_HIP_ROLL] += hipCorrectionY_() * sin(pendulum_.getTimePercentage() * M_PI);
    }
    else
    {
      rLegAngles[JOINTS_R_LEG::R_HIP_ROLL] -= hipCorrectionY_() * sin(pendulum_.getTimePercentage() * M_PI);
    }
  }

  for (unsigned int i = 0; i < JOINTS_L_LEG::L_LEG_MAX; i++)
  {
    bodyAngles[JOINTS::L_HIP_YAW_PITCH + i] = lLegAngles[i];
    bodyAngles[JOINTS::R_HIP_YAW_PITCH + i] = rLegAngles[i];
  }
}

/** getTorsoMatrixChange **/
Pose WalkingEngine::getTorsoMatrixChange()
{
  return pendulum_.getTorsoMatrixChange();
}

void WalkingEngine::countPose()
{
  if (poseCountFinish_ <= 0)
  {
    countPose_ = false;
  }
  else
  {
    poseCountFinish_ -= 10;
  }
}

void WalkingEngine::reportOdometry()
{
  // If the support foot change, the the private member is update (dirty) - thus one looses the odometry for this cycle
  if (lastSupport_ != pendulum_.getSupport())
  {
    lastSupport_ = pendulum_.getSupport();
    projectedTorsoPosition_ = getProjectedTorsoPostion();
    // Compensate for lost torso shift by simply taking the shift of the last cycle
    pushOdometryUpdate(lastShift_ / 1000);
  }
  // Get the new projected torso
  Vector2f newProjectedTorsoPosition = getProjectedTorsoPostion();
  // Calculate the shift form the difference of the torso matrices
  Vector2f projectedShift = newProjectedTorsoPosition - projectedTorsoPosition_;
  projectedTorsoPosition_ = newProjectedTorsoPosition;
  lastShift_ = projectedShift;

  // Push back the odometry into the result queue
  pushOdometryUpdate(projectedShift / 1000);
}

Vector2f WalkingEngine::getProjectedTorsoPostion()
{
  // Rotate with IMU measurement to take torso tilt into account
  // TODO: directly calculate imuInverse
  const Vector3f& angle = imuSensorData_->angle;
  KinematicMatrix imu = KinematicMatrix::rotY(angle.y()) * KinematicMatrix::rotX(angle.x());
  // the position of the torso measured from the current support foot
  Vector3f measuredTorso2support = imu.invert() * (pendulum_.getSupport() == SF_LEFT_SUPPORT ? robotKinematics_->matrices[JOINTS::L_FOOT].invert().posV
                                                                                             : robotKinematics_->matrices[JOINTS::R_FOOT].invert().posV);
  return {measuredTorso2support.x(), measuredTorso2support.y()};
}

void WalkingEngine::applyAnkleController(std::vector<float>& bodyAngles)
{
  bodyAngles[JOINTS::L_ANKLE_PITCH] += ankleAccumulator_ / 25;
  bodyAngles[JOINTS::R_ANKLE_PITCH] += ankleAccumulator_ / 25;
}

void WalkingEngine::generateStandAngles()
{
  ComPosition standComCommand;
  pendulum_.computeStandCom(standComCommand);
  computeLegAngles2Com(standAngles_, standComCommand, 0, 0, true);
}
