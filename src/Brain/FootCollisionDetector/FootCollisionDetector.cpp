#include "Brain/FootCollisionDetector/FootCollisionDetector.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Chronometer.hpp"
#include <numeric>

FootCollisionDetector::FootCollisionDetector(const ModuleManagerInterface& manager)
  : Module(manager)
  , timeHoldState_(*this, "timeHoldState", [] {})
  , timeHoldCollision_(*this, "timeHoldCollision", [] {})
  , buttonData_(*this)
  , cycleInfo_(*this)
  , bodyDamageData_(*this)
  , footCollisionData_(*this)
{
  resetCollisionState();
}

void FootCollisionDetector::cycle()
{
  {
    Chronometer time(debug(), mount_ + ".cycle_time");
    if (bodyDamageData_->damagedSwitches[BodySwitches::L_FOOT_LEFT] ||
        bodyDamageData_->damagedSwitches[BodySwitches::L_FOOT_RIGHT] ||
        bodyDamageData_->damagedSwitches[BodySwitches::R_FOOT_LEFT] ||
        bodyDamageData_->damagedSwitches[BodySwitches::R_FOOT_RIGHT])
    {
      return;
    }
    getFootBumperState();
    updateCollisionState();
    // Reset collision state if start of bumper collision sequence was too long ago
    if (collisionState_ != CollisionState::WAIT &&
        cycleInfo_->getAbsoluteTimeDifference(timeBumpSequenceBegin_) > timeHoldState_())
    {
      resetCollisionState();
    }
    holdCollision();
    footCollisionData_->valid = true;
    // Prepare data for next cycle
    lastFootSide_ = currentFootSide_;
  }
  sendDebug();
}

void FootCollisionDetector::getFootBumperState()
{
  const auto isLeftBumped{buttonData_->switches.isLeftFootLeftPressed ||
                          buttonData_->switches.isLeftFootRightPressed};
  const auto isRightBumped{buttonData_->switches.isRightFootLeftPressed ||
                           buttonData_->switches.isRightFootRightPressed};
  if (isLeftBumped && isRightBumped)
  {
    currentFootSide_ = Side::BOTH;
    timeCurrentBumper_ = cycleInfo_->startTime;
  }
  else if (isLeftBumped)
  {
    currentFootSide_ = Side::LEFT;
    timeCurrentBumper_ = cycleInfo_->startTime;
  }
  else if (isRightBumped)
  {
    currentFootSide_ = Side::RIGHT;
    timeCurrentBumper_ = cycleInfo_->startTime;
  }
}

void FootCollisionDetector::updateCollisionState()
{
  switch (collisionState_)
  {
    case CollisionState::WAIT:
      if (currentFootSide_ != Side::NONE)
      {
        timeBumpSequenceBegin_ = timeCurrentBumper_;
        collisionState_ = CollisionState::TRIGGERED_ONCE;
      }
      break;
    case CollisionState::TRIGGERED_ONCE:
      if (hasFootCollisionOnOtherFoot())
      {
        collisionState_ = CollisionState::TRIGGERED_TWICE;
      }
      break;
    case CollisionState::TRIGGERED_TWICE:
      if (hasFootCollisionOnOtherFoot())
      {
        timeLastCollision_ = cycleInfo_->startTime;
        footCollisionData_->timestamp = timeLastCollision_;
        resetCollisionState();
      }
      break;
    default:
      assert(false);
      break;
  }
}

bool FootCollisionDetector::hasFootCollisionOnOtherFoot()
{
  return cycleInfo_->getAbsoluteTimeDifference(timeBumpSequenceBegin_) < timeHoldState_() &&
         ((lastFootSide_ == Side::LEFT && currentFootSide_ == Side::RIGHT) ||
          (lastFootSide_ == Side::RIGHT && currentFootSide_ == Side::LEFT) ||
          (lastFootSide_ == Side::LEFT && currentFootSide_ == Side::BOTH) ||
          (lastFootSide_ == Side::RIGHT && currentFootSide_ == Side::BOTH) ||
          (lastFootSide_ == Side::BOTH && currentFootSide_ == Side::RIGHT) ||
          (lastFootSide_ == Side::BOTH && currentFootSide_ == Side::LEFT) ||
          (lastFootSide_ == Side::BOTH && currentFootSide_ == Side::BOTH));
}

void FootCollisionDetector::holdCollision()
{
  if (cycleInfo_->getAbsoluteTimeDifference(timeLastCollision_) < timeHoldCollision_())
  {
    footCollisionData_->collision = true;
  }
}

void FootCollisionDetector::resetCollisionState()
{
  currentFootSide_ = Side::NONE;
  collisionState_ = CollisionState::WAIT;
  timeBumpSequenceBegin_ = Clock::time_point{};
}

void FootCollisionDetector::sendDebug()
{
  debug().update(mount_ + ".leftFoot", buttonData_->switches.isLeftFootLeftPressed ||
                                           buttonData_->switches.isLeftFootRightPressed);
  debug().update(mount_ + ".rightFoot", buttonData_->switches.isRightFootLeftPressed ||
                                            buttonData_->switches.isRightFootRightPressed);
  debug().update(mount_ + ".leftButtonLeftFoot", buttonData_->switches.isLeftFootLeftPressed);
  debug().update(mount_ + ".rightButtonLeftFoot", buttonData_->switches.isLeftFootRightPressed);
  debug().update(mount_ + ".leftButtonRightFoot", buttonData_->switches.isRightFootLeftPressed);
  debug().update(mount_ + ".rightButtonRightFoot", buttonData_->switches.isRightFootRightPressed);
}
