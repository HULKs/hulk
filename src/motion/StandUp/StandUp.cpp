/// Module with standup-routines for tuhhSDK
/**
 * @author Jan Plitzkow
 */

#include <cmath>

#include <Definitions/keys.h>
#include <Modules/NaoProvider.h>
#include <Modules/Poses.h>
#include <Tools/Kinematics/InverseKinematics.h>
#include <Tools/Time.hpp>

#include "StandUp.hpp"
#include "print.hpp"


StandUp::StandUp(const ModuleManagerInterface& manager)
  : Module(manager)
  , angleTolSideCheck_(*this, "angleTolSideCheck")
  , angleTolFmPoseCheck_(*this, "angleTolFmPoseCheck")
  , angleTolSuccessCheck_(*this, "angleTolSuccessCheck")
  , checkingGroundSideInterval_(*this, "checkingGroundSideInterval")
  , maxNumRepeatedSideChecks_(*this, "maxNumRepeatedSideChecks")
  , defaultSideIfCheckFail_(*this, "defaultSideIfCheckFail")
  , checkingSuccessInterval_(*this, "checkingSuccessInterval")
  , maxNumRepeatedSuccessChecks_(*this, "maxNumRepeatedSuccessChecks")
  , standUpMotionFootSpeed_(*this, "standUpMotionFootSpeed")
  , standUpBackMotionFile_(*this, "standUpBackMotionFile")
  , standUpFrontMotionFile_(*this, "standUpFrontMotionFile")
  , motionRequest_(*this)
  , motionActivation_(*this)
  , cycleInfo_(*this)
  , imuSensorData_(*this)
  , jointSensorData_(*this)
  , gameControllerState_(*this)
  , standUpResult_(*this)
  , standUpOutput_(*this)
  , status_(Status::IDLE)
  , numSideChecks_(0)
  , numSuccessChecks_(0)
  , timerClock_(0)
  , finalPose_(Poses::getPose(Poses::READY))
  , standUpMotionBack_(*cycleInfo_, *jointSensorData_)
  , standUpMotionFront_(*cycleInfo_, *jointSensorData_)
  , interpolator_()
  , leftArmInterpolatorFirstStage_()
  , leftArmInterpolatorSecondStage_()
  , rightArmInterpolatorFirstStage_()
  , rightArmInterpolatorSecondStage_()
{
  print("standUp: Initializing module...", LogLevel::INFO);

  /// \li Read motion player files and add the final pose
  standUpMotionFront_.loadFromFile(robotInterface().getFileRoot() + "motions/" +
                                   standUpFrontMotionFile_());
  standUpMotionBack_.loadFromFile(robotInterface().getFileRoot() + "motions/" +
                                  standUpBackMotionFile_());
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
      std::stringstream str;
      str << "standUp: CheckLayingSide is UNDEFINED. " << (numSideChecks_ + 1)
          << ". try to force defined position...";
      print(str.str(), LogLevel::INFO);
      interpolator_.reset(jointSensorData_->getBodyAngles(), finalPose_,
                          checkingGroundSideInterval_() * 0.9);
      timerClock_ = checkingGroundSideInterval_();
    }
  }
  else
  {
    startActualStandUp(groundSide);
  }
  return;
}

StandUp::Side StandUp::getLayingSide(const float angleTol)
{
  const Vector3f& angleData = imuSensorData_->angle;
  print("standUp: get LayingSide angleData x:", angleData.x() / TO_RAD, LogLevel::DEBUG);
  print("standUp: get LayingSide angleData y:", angleData.y() / TO_RAD, LogLevel::DEBUG);
  print("standUp: get LayingSide angleData z:", angleData.z() / TO_RAD, LogLevel::DEBUG);

  if ((fabs(angleData.x()) < angleTol * TO_RAD) && (fabs(angleData.y()) < angleTol * TO_RAD))
  {
    return Side::FOOT;
  }
  else if ((fabs(angleData.x()) < angleTol * TO_RAD) &&
           (fabs(angleData.y() - 90 * TO_RAD) < angleTol * TO_RAD))
  {
    return Side::FRONT;
  }
  else if ((fabs(angleData.x()) < angleTol * TO_RAD) &&
           (fabs(angleData.y() + 90 * TO_RAD) < angleTol * TO_RAD))
  {
    return Side::BACK;
  }
  else
  {
    return Side::UNDEFINED;
  }
}

void StandUp::startActualStandUp(Side groundSide)
{
  status_ = Status::STANDING_UP;
  numSideChecks_ = 0;
  // initiate movement
  switch (groundSide)
  {
    case Side::BACK:
      print("standUp: MotionBack from BACK starting...", LogLevel::INFO);
      timerClock_ = standUpMotionBack_.play();
      break;
    case Side::FRONT:
      print("standUp: MotionFront from FRONT starting...", LogLevel::INFO);
      timerClock_ = standUpMotionFront_.play();
      break;
    case Side::FOOT:
      print("standUp: Motion from FOOT starting...", LogLevel::INFO);
      timerClock_ = standUpMotionFoot();
      break;
    default:
      print("standUp: performStandup() called with unknown ground side...", LogLevel::ERROR);
      timerClock_ = 0;
  }
}

bool StandUp::isActive()
{
  return (status_ != Status::IDLE);
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
  else if (motionActivation_->activeMotion == MotionRequest::BodyMotion::STAND_UP &&
           motionActivation_
                   ->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::STAND_UP)] >
               0.9)
  {
    standUp();
  }

  standUpOutput_->angles = Poses::getPose(Poses::READY);
  standUpOutput_->stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 0.7f);
  switch (status_)
  {
    case Status::PREPARING:
    {
      if (!interpolator_.finished())
      {
        std::vector<float> angles = interpolator_.step(10);
        standUpOutput_->angles = angles;
        standUpOutput_->stiffnesses = std::vector<float>(angles.size(), 1.f);
      }

      timerClock_ = timerClock_ - 10;
      if ((timerClock_ % 100) == 0)
      {
        print("standUp: PREPARING: Remaining Time: ", timerClock_, LogLevel::DEBUG);
      }
      if (timerClock_ <= 0)
      {
        prepareStandUp();
      }
      break;
    }
    case Status::STANDING_UP:
    {
      /// only to be sure that stiffness is set during standup
      MotionFilePlayer::JointValues values;
      bool send = false;
      if (standUpMotionBack_.isPlaying())
      {
        values = standUpMotionBack_.cycle();
        send = true;
      }
      else if (standUpMotionFront_.isPlaying())
      {
        values = standUpMotionFront_.cycle();
        send = true;
      }
      else if (!interpolator_.finished())
      {
        values.angles = interpolator_.step(10);
        std::vector<float> arms;
        if (!leftArmInterpolatorFirstStage_.finished())
        {
          arms = leftArmInterpolatorFirstStage_.step(10);
        }
        else
        {
          arms = leftArmInterpolatorSecondStage_.step(10);
        }
        for (unsigned int i = 0; i < arms.size(); i++)
        {
          values.angles[i + JOINTS::L_SHOULDER_PITCH] = arms[i];
        }
        if (!rightArmInterpolatorFirstStage_.finished())
        {
          arms = rightArmInterpolatorFirstStage_.step(10);
        }
        else
        {
          arms = rightArmInterpolatorSecondStage_.step(10);
        }
        for (unsigned int i = 0; i < arms.size(); i++)
        {
          values.angles[i + JOINTS::R_SHOULDER_PITCH] = arms[i];
        }
        values.stiffnesses = std::vector<float>(values.angles.size(), 1.f);
        send = true;
      }
      if (send)
      {
        standUpOutput_->angles = values.angles;
        standUpOutput_->stiffnesses = values.stiffnesses;
      }

      timerClock_ = timerClock_ - 10;
      if ((timerClock_ % 100) == 0)
      {
        print("standUp: STANDING_UP: Remaining Time: ", timerClock_, LogLevel::DEBUG);
      }
      if (timerClock_ <= 0)
      {
        checkSuccess();
      }
      break;
    }
    default:
      resetStandUp();
  }
  if (status_ == Status::IDLE)
  {
    standUpOutput_->safeExit = true;
  }
}

void StandUp::checkSuccess()
{

  timerClock_ = 0;
  if (getLayingSide(angleTolSuccessCheck_()) == Side::FOOT)
  {
    print("standUp: Standup finished successfully.", LogLevel::INFO);

    standUpResult_->finishedSuccessfully = true;

    resetStandUp();
  }
  else
  {
    numSuccessChecks_++;
    if (numSuccessChecks_ > maxNumRepeatedSuccessChecks_())
    {
      print("standUp: Standup finished without success.", LogLevel::INFO);

      resetStandUp();
    }
    else
    {
      print("standUp: Short waiting for success", LogLevel::INFO);
      timerClock_ = 100;
    }
  }
}

void StandUp::resetStandUp()
{
  timerClock_ = 0;
  numSideChecks_ = 0;
  numSuccessChecks_ = 0;
  status_ = Status::IDLE;
}

int StandUp::standUpMotionFoot()
{
  std::vector<float> vecDiff = jointSensorData_->getBodyAngles();
  float sum = 0; // quadratic sum over difference vector
  for (unsigned int i = 0; i < vecDiff.size(); i++)
  {
    vecDiff.at(i) =
        finalPose_.at(i) - vecDiff.at(i); // difference vector between current and target pose
    sum += (vecDiff.at(i)) * (vecDiff.at(i));
  }
  int time = sum * standUpMotionFootSpeed_() * 200; // using time depending on way-length
  print("standUp: Footmotion time:", time, LogLevel::DEBUG);
  interpolator_.reset(jointSensorData_->getBodyAngles(), finalPose_, time);

  // Special commands for the arms to prevent body collision
  std::vector<float> vecPenalized = Poses::getPose(Poses::PENALIZED);
  std::vector<float> rArmCommands = std::vector<float>(JOINTS_R_ARM::R_ARM_MAX, 0);
  std::vector<float> lArmCommands = std::vector<float>(JOINTS_L_ARM::L_ARM_MAX, 0);

  getArmCommandsFromPose(vecPenalized, rArmCommands, lArmCommands);
  leftArmInterpolatorFirstStage_.reset(jointSensorData_->getLArmAngles(), lArmCommands, time / 2);
  rightArmInterpolatorFirstStage_.reset(jointSensorData_->getRArmAngles(), rArmCommands, time / 2);
  std::vector<float> prevLArmCommands = lArmCommands;
  std::vector<float> prevRArmCommands = rArmCommands;

  getArmCommandsFromPose(finalPose_, rArmCommands, lArmCommands);
  leftArmInterpolatorSecondStage_.reset(prevLArmCommands, lArmCommands, time / 2);
  rightArmInterpolatorSecondStage_.reset(prevRArmCommands, rArmCommands, time / 2);

  return time;
}

void StandUp::getArmCommandsFromPose(const std::vector<float>& pose,
                                     std::vector<float>& rArmCommands,
                                     std::vector<float>& lArmCommands)
{
  for (int i = 0; i < JOINTS_L_ARM::L_ARM_MAX; i++)
  {
    lArmCommands[i] = pose[JOINTS::L_SHOULDER_PITCH + i];
    rArmCommands[i] = pose[JOINTS::R_SHOULDER_PITCH + i];
  }
}
