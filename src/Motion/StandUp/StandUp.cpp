#include "Motion/StandUp/StandUp.hpp"
#include "Framework/Log/Log.hpp"
#include "Hardware/Clock.hpp"
#include "Hardware/JointUtils.hpp"
#include <cmath>
#include <type_traits>


StandUp::StandUp(const ModuleManagerInterface& manager)
  : Module{manager}
  , actionCommand_{*this}
  , motionActivation_{*this}
  , cycleInfo_{*this}
  , imuSensorData_{*this}
  , jointSensorData_{*this}
  , gameControllerState_{*this}
  , poses_{*this}
  , standUpResult_{*this}
  , standUpOutput_{*this}
  , angleTolSideCheck_{*this, "angleTolSideCheck"}
  , angleTolFmPoseCheck_{*this, "angleTolFmPoseCheck"}
  , angleTolSuccessCheck_{*this, "angleTolSuccessCheck"}
  , checkingGroundSideInterval_{*this, "checkingGroundSideInterval"}
  , maxNumRepeatedSideChecks_{*this, "maxNumRepeatedSideChecks"}
  , defaultSideIfCheckFail_{*this, "defaultSideIfCheckFail"}
  , checkingSuccessInterval_{*this, "checkingSuccessInterval"}
  , maxNumRepeatedSuccessChecks_{*this, "maxNumRepeatedSuccessChecks"}
  , standUpMotionFootSpeed_{*this, "standUpMotionFootSpeed"}
  , standUpBackMotionFile_{*this, "standUpBackMotionFile"}
  , standUpFrontMotionFile_{*this, "standUpFrontMotionFile"}
  , standUpMotionBack_{*cycleInfo_, *jointSensorData_}
  , standUpMotionFront_{*cycleInfo_, *jointSensorData_}
{
  Log<M_MOTION>(LogLevel::INFO) << "standUp: Initializing module...";

  /// \li Read motion player files and add the final pose
  standUpMotionFront_.loadFromFile(robotInterface().getFileRoot() + "motions/" +
                                   standUpFrontMotionFile_());
  standUpMotionBack_.loadFromFile(robotInterface().getFileRoot() + "motions/" +
                                  standUpBackMotionFile_());
}

void StandUp::cycle()
{
  if (gameControllerState_->gameState == GameState::INITIAL)
  {
    // It does not make any sense to be fallen in the initial state. The robot should stand when it
    // exits this state anyway.
    resetStandUp();
    standUpResult_->finishedSuccessfully = true;
  }
  else if (motionActivation_->activeMotion == ActionCommand::Body::MotionType::STAND_UP &&
           motionActivation_->activations[ActionCommand::Body::MotionType::STAND_UP] > .9f)
  {
    standUp();
  }

  standUpOutput_->angles = poses_->angles[Poses::Type::READY];
  standUpOutput_->stiffnesses.fill(0.7f);
  switch (status_)
  {
    case Status::PREPARING: {
      if (!interpolator_.isFinished())
      {
        standUpOutput_->angles = {interpolator_.step(cycleInfo_->cycleTime)};
        standUpOutput_->stiffnesses.fill(1.0f);
      }

      timerClock_ = timerClock_ - 10ms;
      if (timerClock_ <= 0s)
      {
        prepareStandUp();
      }
      break;
    }
    case Status::STANDING_UP: {
      /// only to be sure that stiffness is set during standup
      JointsArray<float> angles;
      JointsArray<float> stiffnesses;
      bool send = false;
      if (standUpMotionBack_.isPlaying())
      {
        const auto cycle = standUpMotionBack_.cycle();
        angles = cycle.angles;
        stiffnesses = cycle.stiffnesses;
        send = true;
      }
      else if (standUpMotionFront_.isPlaying())
      {
        const auto cycle = standUpMotionFront_.cycle();
        angles = cycle.angles;
        stiffnesses = cycle.stiffnesses;
        send = true;
      }
      else if (!interpolator_.isFinished())
      {
        angles = {interpolator_.step(cycleInfo_->cycleTime)};
        const auto leftArm = !leftArmInterpolatorFirstStage_.isFinished()
                                 ? leftArmInterpolatorFirstStage_.step(cycleInfo_->cycleTime)
                                 : leftArmInterpolatorSecondStage_.step(cycleInfo_->cycleTime);

        const auto rightArm = !rightArmInterpolatorFirstStage_.isFinished()
                                  ? rightArmInterpolatorFirstStage_.step(cycleInfo_->cycleTime)
                                  : rightArmInterpolatorSecondStage_.step(cycleInfo_->cycleTime);

        JointUtils::fillArms(angles, JointsArmArray<float>{leftArm},
                             JointsArmArray<float>{rightArm});
        stiffnesses.fill(1.f);
        send = true;
      }
      if (send)
      {
        standUpOutput_->angles = angles;
        standUpOutput_->stiffnesses = stiffnesses;
      }

      timerClock_ = timerClock_ - 10ms;
      if (timerClock_ <= 0s)
      {
        checkSuccess();
      }
      break;
    }
    default:
      resetStandUp();
  }
  standUpOutput_->safeExit = status_ == Status::IDLE;
}

void StandUp::standUp()
{
  if (!isActive())
  {
    // go into status PREPARING and connect the cycle
    status_ = Status::PREPARING;
    prepareStandUp();
  }
}

void StandUp::prepareStandUp()
{
  // get the side of the Nao which is at the downside
  Side groundSide = getLayingSide(angleTolSideCheck_());

  if (groundSide == Side::UNDEFINED)
  {
    numSideChecks_++;
    if (numSideChecks_ > maxNumRepeatedSideChecks_())
    {
      // default standup motion after side check returned UNDEFINED to often
      startActualStandUp(defaultSideIfCheckFail_());
    }
    else
    {
      // go to ready position in order to flip the Nao to a defined position
      Log<M_MOTION>(LogLevel::INFO) << "standUp: CheckLayingSide is UNDEFINED. "
                                    << (numSideChecks_ + 1) << ". try to force defined position...";
      interpolator_.reset(jointSensorData_->getBodyAngles(), poses_->angles[Poses::Type::READY],
                          checkingGroundSideInterval_() * 0.9f);
      timerClock_ = checkingGroundSideInterval_();
    }
  }
  else
  {
    startActualStandUp(groundSide);
  }
}

StandUp::Side StandUp::getLayingSide(const float angleTol)
{
  const Vector2f& angleData = imuSensorData_->angle;
  Log<M_MOTION>(LogLevel::DEBUG) << "standUp: get LayingSide angleData x:" << angleData.x() / TO_RAD
                                 << " y:" << angleData.y() / TO_RAD;

  if ((std::abs(angleData.x()) < angleTol * TO_RAD) &&
      (std::abs(angleData.y()) < angleTol * TO_RAD))
  {
    return Side::FOOT;
  }
  if ((std::abs(angleData.x()) < angleTol * TO_RAD) &&
      (std::abs(angleData.y() - 90 * TO_RAD) < angleTol * TO_RAD))
  {
    return Side::FRONT;
  }
  if ((std::abs(angleData.x()) < angleTol * TO_RAD) &&
      (std::abs(angleData.y() + 90 * TO_RAD) < angleTol * TO_RAD))
  {
    return Side::BACK;
  }
  return Side::UNDEFINED;
}

void StandUp::startActualStandUp(Side groundSide)
{
  status_ = Status::STANDING_UP;
  numSideChecks_ = 0;
  // initiate movement
  switch (groundSide)
  {
    case Side::BACK:
      Log<M_MOTION>(LogLevel::INFO) << "standUp: MotionBack from BACK starting...";
      timerClock_ = std::chrono::milliseconds{standUpMotionBack_.play()};
      break;
    case Side::FRONT:
      Log<M_MOTION>(LogLevel::INFO) << "standUp: MotionFront from FRONT starting...";
      timerClock_ = std::chrono::milliseconds{standUpMotionFront_.play()};
      break;
    case Side::FOOT:
      Log<M_MOTION>(LogLevel::INFO) << "standUp: Motion from FOOT starting...";
      timerClock_ = standUpMotionFoot();
      break;
    default:
      Log<M_MOTION>(LogLevel::INFO)
          << "standUp: performStandup() called with unknown ground side...";
      timerClock_ = 0s;
  }
}

bool StandUp::isActive()
{
  return (status_ != Status::IDLE);
}

void StandUp::checkSuccess()
{

  timerClock_ = 0s;
  if (getLayingSide(angleTolSuccessCheck_()) == Side::FOOT)
  {
    Log<M_MOTION>(LogLevel::INFO) << "standUp: Standup finished successfully.";

    standUpResult_->finishedSuccessfully = true;

    resetStandUp();
  }
  else
  {
    numSuccessChecks_++;
    if (numSuccessChecks_ > maxNumRepeatedSuccessChecks_())
    {
      Log<M_MOTION>(LogLevel::INFO) << "standUp: Standup finished without success.";

      resetStandUp();
    }
    else
    {
      Log<M_MOTION>(LogLevel::INFO) << "standUp: Short waiting for success";
      timerClock_ = 100ms;
    }
  }
}

void StandUp::resetStandUp()
{
  timerClock_ = 0s;
  numSideChecks_ = 0;
  numSuccessChecks_ = 0;
  status_ = Status::IDLE;
}

Clock::duration StandUp::standUpMotionFoot()
{
  auto vecDiff = jointSensorData_->getBodyAngles();
  float sum = 0.f; // quadratic sum over difference vector
  for (unsigned int i = 0; i < vecDiff.size(); i++)
  {
    vecDiff.at(i) = poses_->angles[Poses::Type::READY].at(i) -
                    vecDiff.at(i); // difference vector between current and target pose
    sum += (vecDiff.at(i)) * (vecDiff.at(i));
  }
  static_assert(std::is_same_v<Clock::duration::period, std::chrono::seconds::period>);
  auto time =
      Clock::duration{sum * standUpMotionFootSpeed_() * 0.2f}; // using time depending on way-length
  Log<M_MOTION>(LogLevel::DEBUG) << "StandUp: Footmotion time: " << time.count();
  interpolator_.reset(jointSensorData_->getBodyAngles(), poses_->angles[Poses::Type::READY], time);

  // Special commands for the arms to prevent body collision
  const auto penalized = poses_->angles[Poses::Type::PENALIZED];
  const auto lArmCommands = JointUtils::extractLeftArm(penalized);
  const auto rArmCommands = JointUtils::extractRightArm(penalized);
  leftArmInterpolatorFirstStage_.reset(jointSensorData_->getLArmAngles(), lArmCommands, time / 2.f);
  rightArmInterpolatorFirstStage_.reset(jointSensorData_->getRArmAngles(), rArmCommands,
                                        time / 2.f);
  leftArmInterpolatorSecondStage_.reset(
      lArmCommands, JointUtils::extractLeftArm(poses_->angles[Poses::Type::READY]), time / 2.f);
  rightArmInterpolatorSecondStage_.reset(
      rArmCommands, JointUtils::extractRightArm(poses_->angles[Poses::Type::READY]), time / 2.f);

  return time;
}
