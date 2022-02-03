#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/Poses.hpp"
#include "Data/StandUpOutput.hpp"
#include "Data/StandUpResult.hpp"
#include "Framework/Module.hpp"
#include "Hardware/Clock.hpp"
#include "Motion/Utils/Interpolator/Interpolator.hpp"
#include "Motion/Utils/MotionFile/MotionFilePlayer.hpp"
#include <string>

class Motion;

/**
 * @brief The StandUp class implements a StandUp motion for the NAO.
 */
class StandUp : public Module<StandUp, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"StandUp"};
  /**
   * @brief StandUp initializes members and loads motion files
   * @param manager a reference to motion
   */
  explicit StandUp(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks for a new command and initiates a stand up motion if needed
   */
  void cycle() override;

  /**
   * @brief Side is an enum to specify different ground sides
   */
  enum class Side
  {
    FRONT,
    BACK,
    FOOT,
    UNDEFINED
  };

private:
  /**
   * @brief Status is an enum to specify the status of the StandUp module
   */
  enum class Status
  {
    IDLE,
    PREPARING,
    STANDING_UP
  };

  const Dependency<ActionCommand> actionCommand_;
  /// a reference to the motion activation of last cycle
  const Reference<MotionActivation> motionActivation_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<IMUSensorData> imuSensorData_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<Poses> poses_;

  Production<StandUpResult> standUpResult_;
  Production<StandUpOutput> standUpOutput_;

  /// tolerance of body angle data in degrees when determining ground side
  const Parameter<float> angleTolSideCheck_;
  /// tolerance of body angle data in degrees when determining FmPose
  const Parameter<float> angleTolFmPoseCheck_;
  /// tolerance of body angle data in degrees when determining success
  const Parameter<float> angleTolSuccessCheck_;
  /// time between two checks of ground side if UNDEFINED
  const Parameter<Clock::duration> checkingGroundSideInterval_;
  /// maximum number of checking side with result UNDEFINED
  const Parameter<unsigned int> maxNumRepeatedSideChecks_;
  /// default standup motion after side check returned UNDEFINED to often
  const Parameter<Side> defaultSideIfCheckFail_;
  /// time till next success check if not yet successful
  const Parameter<Clock::duration> checkingSuccessInterval_;
  /// maximum number of waitings for success (otherwise no success)
  const Parameter<unsigned int> maxNumRepeatedSuccessChecks_;
  /// speed factor for standUp motion from FOOT
  const Parameter<float> standUpMotionFootSpeed_;
  /// name of motion file containing the needed motion for standing up from back side
  const Parameter<std::string> standUpBackMotionFile_;
  /// name of motion file containing the needed motion for standing up from front side
  const Parameter<std::string> standUpFrontMotionFile_;

  /// state of the StandUp-module
  Status status_{Status::IDLE};
  /// number of performed side checks
  unsigned int numSideChecks_{0};
  /// number of performed success checks
  unsigned int numSuccessChecks_{0};
  /// counter of backwards running clock for waiting
  Clock::duration timerClock_;
  /// motion-object for whole standup motion if lying on the back side
  MotionFilePlayer standUpMotionBack_;
  /// motion-object for whole standup motion if lying on the front side
  MotionFilePlayer standUpMotionFront_;
  /// an interpolator
  Interpolator<Clock::duration, static_cast<std::size_t>(Joints::MAX)> interpolator_;
  /// an interpolator
  Interpolator<Clock::duration, static_cast<std::size_t>(JointsArm::MAX)>
      leftArmInterpolatorFirstStage_;
  /// an interpolator
  Interpolator<Clock::duration, static_cast<std::size_t>(JointsArm::MAX)>
      leftArmInterpolatorSecondStage_;
  /// an interpolator
  Interpolator<Clock::duration, static_cast<std::size_t>(JointsArm::MAX)>
      rightArmInterpolatorFirstStage_;
  /// an interpolator
  Interpolator<Clock::duration, static_cast<std::size_t>(JointsArm::MAX)>
      rightArmInterpolatorSecondStage_;

  /**
   * @brief standUp is called when a stand up command arrives and starts the operation of the
   * StandUp module
   */
  void standUp();
  /**
   * @brief isActive checks if there is already a motion being executed
   * @return whether the module is active
   */
  bool isActive();
  /**
   * @brief getLayingSide determines the side on which the NAO lies
   * @param angleTol the maximum allowed difference to a fixed pose
   * @return the side on which the NAO lies
   */
  Side getLayingSide(float angleTol);
  /**
   * @brief prepareStandUp prepares the NAO for standing up
   */
  void prepareStandUp();
  /**
   * @brief startActualStandUp sends joint angle commands for a stand up motion
   * @param groundSide the (previously determined) side from which to stand up
   */
  void startActualStandUp(Side groundSide);
  /**
   * @brief standUpMotionFoot does a stand up motion, provided that the NAO is already on its feet
   * and upright
   * @return time until the motion will be finished
   */
  Clock::duration standUpMotionFoot();
  /**
   * @brief checkSuccess checks if the stand up motion was successful
   */
  void checkSuccess();
  /**
   * @brief resetStandUp resets the member variables that keep state
   */
  void resetStandUp();
};

/**
 * @brief operator>> is needed for Parameter<Side>
 */
inline void operator>>(const Uni::Value& in, StandUp::Side& out)
{
  out = static_cast<StandUp::Side>(in.asInt32());
}
