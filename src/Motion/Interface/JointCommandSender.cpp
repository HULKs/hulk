#include "Motion/Interface/JointCommandSender.hpp"
#include "Framework/Log/Log.hpp"
#include "Hardware/JointUtils.hpp"

JointCommandSender::JointCommandSender(const ModuleManagerInterface& manager)
  : Module{manager}
  , motionActivation_{*this}
  , fallManagerOutput_{*this}
  , headMotionOutput_{*this}
  , jumpOutput_{*this}
  , kickOutput_{*this}
  , pointOutput_{*this}
  , standUpOutput_{*this}
  , sitDownOutput_{*this}
  , sitUpOutput_{*this}
  , walkGeneratorOutput_{*this}
  , jointSensorData_{*this}
  , jointCalibrationData_{*this}
  , bodyDamageData_{*this}
  , puppetMotionOutput_{*this}
  , poses_{*this}
  , motionState_{*this}
{
}

void JointCommandSender::cycle()
{
  using BodyMotion = ActionCommand::Body::MotionType;
  using HeadMotion = ActionCommand::Head::MotionType;
  using ArmMotion = ActionCommand::Arm::MotionType;

  if (motionActivation_->activeMotion == BodyMotion::DEAD ||
      motionActivation_->activeMotion == BodyMotion::HOLD)
  {
    startInterpolationAngles_ = jointSensorData_->getBodyAngles();
  }

  JointsArray<float> angles{jointSensorData_->getBodyAngles()};
  JointsArray<float> stiffnesses{};

  // handle motion outputs
  if (motionActivation_->activeMotion == BodyMotion::DEAD)
  {
    // This handles the double chest button press which should always take priority.
    stiffnesses.fill(0.f);
    motionState_->bodyMotion = BodyMotion::DEAD;
    motionState_->leftArmMotion = ArmMotion::BODY;
    motionState_->rightArmMotion = ArmMotion::BODY;
    motionState_->headMotion = HeadMotion::BODY;
  }
  else if (motionActivation_->activeMotion == BodyMotion::HOLD)
  {
    // keep the angles from the previous cycle
    stiffnesses.fill(0.9f);
    motionState_->bodyMotion = BodyMotion::HOLD;
    motionState_->leftArmMotion = ArmMotion::BODY;
    motionState_->rightArmMotion = ArmMotion::BODY;
    motionState_->headMotion = HeadMotion::BODY;
  }
  else if (motionActivation_->activeMotion == BodyMotion::PUPPET)
  {
    angles = puppetMotionOutput_->angles;
    stiffnesses = puppetMotionOutput_->stiffnesses;
    motionState_->bodyMotion = BodyMotion::PUPPET;
    motionState_->leftArmMotion = ArmMotion::BODY;
    motionState_->rightArmMotion = ArmMotion::BODY;
    motionState_->headMotion = HeadMotion::BODY;
  }
  else
  {
    // This sum can be < 1 when dead or hold are active.
    const float sum = motionActivation_->activations[BodyMotion::JUMP] +
                      motionActivation_->activations[BodyMotion::KICK] +
                      motionActivation_->activations[BodyMotion::FALL_MANAGER] +
                      motionActivation_->activations[BodyMotion::STAND_UP] +
                      motionActivation_->activations[BodyMotion::SIT_DOWN] +
                      motionActivation_->activations[BodyMotion::SIT_UP] +
                      motionActivation_->activations[BodyMotion::PENALIZED] +
                      motionActivation_->activations[BodyMotion::WALK] +
                      motionActivation_->activations[BodyMotion::STAND];
    for (std::size_t i = 0; i < static_cast<std::size_t>(Joints::MAX); i++)
    {
      const auto joint = static_cast<Joints>(i);
      angles[joint] =
          motionActivation_->activations[BodyMotion::JUMP] * jumpOutput_->angles[joint] +
          motionActivation_->activations[BodyMotion::KICK] * kickOutput_->angles[joint] +
          motionActivation_->activations[BodyMotion::FALL_MANAGER] *
              fallManagerOutput_->angles[joint] +
          motionActivation_->activations[BodyMotion::STAND_UP] * standUpOutput_->angles[joint] +
          motionActivation_->activations[BodyMotion::SIT_DOWN] * sitDownOutput_->angles[joint] +
          motionActivation_->activations[BodyMotion::SIT_UP] * sitUpOutput_->angles[joint] +
          motionActivation_->activations[BodyMotion::PENALIZED] *
              poses_->angles[Poses::Type::PENALIZED][joint] +
          (motionActivation_->activations[BodyMotion::WALK] +
           motionActivation_->activations[BodyMotion::STAND]) *
              walkGeneratorOutput_->angles[joint] +
          (1 - sum) * startInterpolationAngles_[joint]; // This is needed for interpolating from
                                                        // dead or hold.
      // Determine the highest stiffness of all activated motions
      const float penalizedStiffness =
          motionActivation_->activations[BodyMotion::PENALIZED] < 0.9f ? 0.7f : 0.2f;
      const float stiffness = std::max(
          {(motionActivation_->activations[BodyMotion::JUMP] > 0.f)
               ? jumpOutput_->stiffnesses[joint]
               : 0.f,
           (motionActivation_->activations[BodyMotion::KICK] > 0.f)
               ? kickOutput_->stiffnesses[joint]
               : 0.f,
           (motionActivation_->activations[BodyMotion::FALL_MANAGER] > 0.f)
               ? fallManagerOutput_->stiffnesses[joint]
               : 0.f,
           (motionActivation_->activations[BodyMotion::STAND_UP] > 0.f)
               ? standUpOutput_->stiffnesses[joint]
               : 0.f,
           (motionActivation_->activations[BodyMotion::SIT_DOWN] > 0.f)
               ? sitDownOutput_->stiffnesses[joint]
               : 0.f,
           (motionActivation_->activations[BodyMotion::SIT_UP] > 0.f)
               ? sitUpOutput_->stiffnesses[joint]
               : 0.f,
           (motionActivation_->activations[BodyMotion::PENALIZED] > 0.f) ? penalizedStiffness : 0.f,
           (motionActivation_->activations[BodyMotion::WALK] > 0.f)
               ? walkGeneratorOutput_->stiffnesses[joint]
               : 0.f,
           (motionActivation_->activations[BodyMotion::STAND] > 0.f)
               ? walkGeneratorOutput_->stiffnesses[joint]
               : 0.f});
      stiffnesses[joint] = stiffness;
    }
    motionState_->bodyMotion = motionActivation_->activeMotion;
    motionState_->leftArmMotion = ArmMotion::BODY;
    motionState_->rightArmMotion = ArmMotion::BODY;
    motionState_->headMotion = HeadMotion::BODY;
  }
  // The head motion can be trusted that it only wants to send when it is allowed to.
  if (motionActivation_->headMotionActivation > 0.f)
  {
    angles[Joints::HEAD_YAW] =
        (1.f - motionActivation_->headMotionActivation) * angles[Joints::HEAD_YAW] +
        motionActivation_->headMotionActivation * headMotionOutput_->angles[Joints::HEAD_YAW];
    angles[Joints::HEAD_PITCH] =
        (1.f - motionActivation_->headMotionActivation) * angles[Joints::HEAD_PITCH] +
        motionActivation_->headMotionActivation * headMotionOutput_->angles[Joints::HEAD_PITCH];
    stiffnesses[Joints::HEAD_YAW] = headMotionOutput_->stiffnesses[Joints::HEAD_YAW];
    stiffnesses[Joints::HEAD_PITCH] = headMotionOutput_->stiffnesses[Joints::HEAD_PITCH];
    motionState_->headMotion = HeadMotion::ANGLES;
  }
  // The pointer can be trusted, too. Well, actually not.
  if (pointOutput_->wantToSend && motionActivation_->armsCanBeUsed)
  {
    JointUtils::fillArms(angles, pointOutput_->leftAngles, pointOutput_->rightAngles);
    JointUtils::fillArms(stiffnesses, pointOutput_->leftStiffnesses,
                         pointOutput_->rightStiffnesses);
    motionState_->leftArmMotion = ArmMotion::POINT;
    motionState_->rightArmMotion = ArmMotion::POINT;
  }
  // Add the calibration offsets and remove stiffness of damaged joints
  JointsArray<float> calibratedAngles{};
  for (std::size_t i = 0; i < angles.size(); i++)
  {
    const auto joint = static_cast<Joints>(i);
    if (bodyDamageData_->damagedJoints[joint])
    {
      // joint is damaged, set to angle of ready pose
      calibratedAngles[joint] = poses_->angles[Poses::Type::READY][joint];
      stiffnesses[joint] = 0.f;
      continue;
    }
    calibratedAngles[joint] = angles[joint] + jointCalibrationData_->calibrationOffsets[joint];
  }
  motionState_->angles = calibratedAngles;
  motionState_->stiffnesses = stiffnesses;
#ifndef NDEBUG
  for (std::size_t i = 0; i < static_cast<std::size_t>(Joints::MAX); i++)
  {
    const auto joint = static_cast<Joints>(i);
    if (std::isnan(kickOutput_->angles[joint]))
    {
      std::cout << "KickOutput " << JOINT_NAMES[joint] << " was NaN" << '\n';
    }
    if (std::isnan(walkGeneratorOutput_->angles[joint]))
    {
      std::cout << "WalkingEngineWalkOuptut " << JOINT_NAMES[joint] << " was NaN" << '\n';
    }
    if (std::isnan(fallManagerOutput_->angles[joint]))
    {
      std::cout << "FallManagerOutput_ " << JOINT_NAMES[joint] << " was NaN" << '\n';
    }
    if (std::isnan(fallManagerOutput_->angles[joint]))
    {
      std::cout << "FallManagerOutput " << JOINT_NAMES[joint] << " was NaN" << '\n';
    }
    if (std::isnan(standUpOutput_->angles[joint]))
    {
      std::cout << "StandUpOutput " << JOINT_NAMES[joint] << " was NaN" << '\n';
    }
    if (std::isnan(sitDownOutput_->angles[joint]))
    {
      std::cout << "SitDownOutput " << JOINT_NAMES[joint] << " was NaN" << '\n';
    }
    if (std::isnan(sitUpOutput_->angles[joint]))
    {
      std::cout << "SitUpOutput " << JOINT_NAMES[joint] << " was NaN" << '\n';
    }
    assert(!std::isnan(calibratedAngles[joint]));
    assert(!std::isnan(stiffnesses[joint]));

    if (calibratedAngles[joint] < robotMetrics().minRange(joint) ||
        calibratedAngles[joint] > robotMetrics().maxRange(joint))
    {
      Log<M_MOTION>(LogLevel::DEBUG)
          << "Requested angles out of range! Motion "
          << static_cast<unsigned int>(motionActivation_->activeMotion) << " requested "
          << JOINT_NAMES[joint] << " to " << calibratedAngles[joint] << ". Allowed range is ["
          << robotMetrics().minRange(joint) << ", " << robotMetrics().maxRange(joint) << "].";
    }
  }
#endif

  robotInterface().setJointAngles(motionState_->angles);
  robotInterface().setJointStiffnesses(motionState_->stiffnesses);
}
