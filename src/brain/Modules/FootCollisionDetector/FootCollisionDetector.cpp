#include "FootCollisionDetector.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Time.hpp"

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
    bool isOneFootBumperDamaged = false;
    for (unsigned int i = 0; i < BUMPERS::BUMPERS_MAX; ++i)
    {
      isOneFootBumperDamaged = isOneFootBumperDamaged || bodyDamageData_->damagedBumpers[i];
    }
    if (isOneFootBumperDamaged)
    {
      return;
    }
    getFootBumperState();
    updateCollisionState();
    // Reset collision state if start of bumper collision sequence was too long ago
    if (collisionState_ != WAIT &&
        cycleInfo_->getTimeDiff(timeBumpSequenceBegin_) > timeHoldState_())
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
  if ((buttonData_->buttons[keys::sensor::SWITCH_L_FOOT_LEFT] ||
       buttonData_->buttons[keys::sensor::SWITCH_L_FOOT_RIGHT]) &&
      (buttonData_->buttons[keys::sensor::SWITCH_R_FOOT_LEFT] ||
       buttonData_->buttons[keys::sensor::SWITCH_R_FOOT_RIGHT]))
  {
    currentFootSide_ = BOTH;
    timeCurrentBumper_ = TimePoint::getCurrentTime();
  }
  else if (buttonData_->buttons[keys::sensor::SWITCH_L_FOOT_LEFT] ||
           buttonData_->buttons[keys::sensor::SWITCH_L_FOOT_RIGHT])
  {
    currentFootSide_ = LEFT;
    timeCurrentBumper_ = TimePoint::getCurrentTime();
  }
  else if (buttonData_->buttons[keys::sensor::SWITCH_R_FOOT_LEFT] ||
           buttonData_->buttons[keys::sensor::SWITCH_R_FOOT_RIGHT])
  {
    currentFootSide_ = RIGHT;
    timeCurrentBumper_ = TimePoint::getCurrentTime();
  }
}

void FootCollisionDetector::updateCollisionState()
{
  switch (collisionState_)
  {
    case WAIT:
      if (currentFootSide_ != NONE)
      {
        timeBumpSequenceBegin_ = timeCurrentBumper_;
        collisionState_ = TRIGGERED_ONCE;
      }
      break;
    case TRIGGERED_ONCE:
      if (hasFootCollisionOnOtherFoot())
      {
        collisionState_ = TRIGGERED_TWICE;
      }
      break;
    case TRIGGERED_TWICE:
      if (hasFootCollisionOnOtherFoot())
      {
        timeLastCollision_ = TimePoint::getCurrentTime();
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
  return cycleInfo_->getTimeDiff(timeBumpSequenceBegin_) < timeHoldState_() &&
         ((lastFootSide_ == LEFT && currentFootSide_ == RIGHT) ||
          (lastFootSide_ == RIGHT && currentFootSide_ == LEFT) ||
          (lastFootSide_ == LEFT && currentFootSide_ == BOTH) ||
          (lastFootSide_ == RIGHT && currentFootSide_ == BOTH) ||
          (lastFootSide_ == BOTH && currentFootSide_ == RIGHT) ||
          (lastFootSide_ == BOTH && currentFootSide_ == LEFT) ||
          (lastFootSide_ == BOTH && currentFootSide_ == BOTH));
}

void FootCollisionDetector::holdCollision()
{
  if (cycleInfo_->getTimeDiff(timeLastCollision_) < timeHoldCollision_())
  {
    footCollisionData_->collision = true;
  }
}

void FootCollisionDetector::resetCollisionState()
{
  currentFootSide_ = NONE;
  collisionState_ = WAIT;
  timeBumpSequenceBegin_ = TimePoint();
}

void FootCollisionDetector::sendDebug()
{
  debug().update(mount_ + ".leftFoot", (buttonData_->buttons[keys::sensor::SWITCH_L_FOOT_LEFT] ||
                                        buttonData_->buttons[keys::sensor::SWITCH_L_FOOT_RIGHT]) *
                                           1.f);
  debug().update(mount_ + ".rightFoot", (buttonData_->buttons[keys::sensor::SWITCH_R_FOOT_LEFT] ||
                                         buttonData_->buttons[keys::sensor::SWITCH_R_FOOT_RIGHT]) *
                                            1.f);
  debug().update(mount_ + ".leftButtonLeftFoot",
                 buttonData_->buttons[keys::sensor::SWITCH_L_FOOT_LEFT] * 0.5f);
  debug().update(mount_ + ".rightButtonLeftFoot",
                 buttonData_->buttons[keys::sensor::SWITCH_L_FOOT_RIGHT] * 0.5f);
  debug().update(mount_ + ".leftButtonRightFoot",
                 buttonData_->buttons[keys::sensor::SWITCH_R_FOOT_LEFT] * 0.5f);
  debug().update(mount_ + ".rightButtonRightFoot",
                 buttonData_->buttons[keys::sensor::SWITCH_R_FOOT_RIGHT] * 0.5f);
}
