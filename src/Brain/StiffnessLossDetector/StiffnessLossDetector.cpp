#include "Brain/StiffnessLossDetector/StiffnessLossDetector.hpp"
#include "Framework/Log/Log.hpp"
#include "Tools/Math/Angle.hpp"

StiffnessLossDetector::StiffnessLossDetector(const ModuleManagerInterface& manager)
  : Module(manager)
  , jointDiff_(*this)
  , jointSensorData_(*this)
  , motionState_(*this)
  , stiffnessLoss_(*this)
  , disabledJoints_(*this, "disabledJoints", [] {})
  , stiffnessLossAngleThreshold_(*this, "stiffnessLossAngleThreshold",
                                 [this] { stiffnessLossAngleThreshold_() *= TO_RAD; })
  , stiffnessLossCurrentThreshold_(*this, "stiffnessLossCurrentThreshold", [] {})
  , maxNumMisses_(*this, "maxNumMisses", [] {})
  , numHitsForDetection_(*this, "numHitsForDetection", [] {})
{
  assert(disabledJoints_().size() == static_cast<std::size_t>(Joints::MAX));

  stiffnessLossAngleThreshold_() *= TO_RAD;

  hits_.fill(0);
  misses_.fill(0);
}

void StiffnessLossDetector::cycle()
{
  if (!jointDiff_->valid || !jointSensorData_->valid)
  {
    return;
  }

  stiffnessLoss_->stiffnessLoss = false;

  for (std::size_t i = 0; i < jointDiff_->angles.size(); i++)
  {
    if (disabledJoints_()[i])
    {
      continue;
    }

    if (motionState_->bodyMotion == ActionCommand::Body::MotionType::DEAD)
    {
      continue;
    }

    const auto j = static_cast<Joints>(i);
    if (jointDiff_->angles[j] > stiffnessLossAngleThreshold_())
    {
      if (jointSensorData_->currents[j] < stiffnessLossCurrentThreshold_())
      {
        hits_[j]++;
        misses_[j] = 0;
      }
    }
    else
    {
      misses_[j]++;
    }

    if (misses_[j] > maxNumMisses_())
    {
      hits_[j] = 0;
    }

    if (hits_[j] > numHitsForDetection_())
    {
      stiffnessLoss_->stiffnessLoss = true;
      stiffnessLoss_->valid = true;
      debug().playAudio("stiffness_loss_detected", AudioSounds::OUCH);
      Log<M_BRAIN>(LogLevel::INFO)
          << "StiffnessLossDetector: stiffness loss detected in " << JOINT_NAMES[j];
    }
  }
  stiffnessLoss_->valid = true;
}
