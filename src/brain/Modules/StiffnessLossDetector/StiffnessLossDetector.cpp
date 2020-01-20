#include "StiffnessLossDetector.hpp"

#include "Modules/NaoProvider.h"
#include "Tools/Math/Angle.hpp"
#include "print.h"

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
  assert(disabledJoints_().size() == JOINTS::JOINTS_MAX);

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

  for (unsigned int i = 0; i < jointDiff_->angles.size(); i++)
  {
    if (disabledJoints_()[i])
    {
      continue;
    }

    if (motionState_->bodyMotion == MotionRequest::BodyMotion::DEAD)
    {
      continue;
    }

    if (jointDiff_->angles[i] > stiffnessLossAngleThreshold_())
    {
      if (jointSensorData_->currents[i] < stiffnessLossCurrentThreshold_())
      {
        hits_[i]++;
        misses_[i] = 0;
      }
    }
    else
    {
      misses_[i]++;
    }

    if (misses_[i] > maxNumMisses_())
    {
      hits_[i] = 0;
    }

    if (hits_[i] > numHitsForDetection_())
    {
      stiffnessLoss_->stiffnessLoss = true;
      stiffnessLoss_->valid = true;
      debug().playAudio("stiffness_loss_detected", AudioSounds::OUCH);
      Log(LogLevel::INFO) << "StiffnessLossDetector: stiffness loss detected in "
                          << JOINTS::names[i];
    }
  }
  stiffnessLoss_->valid = true;
}
