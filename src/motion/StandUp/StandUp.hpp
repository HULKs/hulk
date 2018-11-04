#pragma once

#include <string>

#include <Data/CycleInfo.hpp>
#include <Data/GameControllerState.hpp>
#include <Data/IMUSensorData.hpp>
#include <Data/JointSensorData.hpp>
#include <Data/MotionActivation.hpp>
#include <Data/MotionRequest.hpp>
#include <Data/StandUpOutput.hpp>
#include <Data/StandUpResult.hpp>
#include <Framework/Module.hpp>
#include <Tools/Kinematics/InverseKinematics.h>

#include "Utils/Interpolator/Interpolator.hpp"
#include "Utils/MotionFile/MotionFilePlayer.hpp"

class Motion;

/**
 * @brief The StandUp class implements a StandUp motion for the NAO.
 * Implementation:
 * @author Jan Plitzkow
 * @author Nicolas Riebesel
 * Configuration:
 * @author Lasse Peters
 */
class StandUp : public Module<StandUp, Motion>
{
public:
  /// the name of this module
  ModuleName name = "StandUp";
  /**
   * @brief StandUp initializes members and loads motion files
   * @param manager a reference to motion
   */
  StandUp(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks for a new command and initiates a stand up motion if needed
   */
  void cycle();

private:
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
  /**
   * @brief Status is an enum to specify the status of the StandUp module
   */
  enum class Status
  {
    IDLE,
    PREPARING,
    STANDING_UP
  };
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
  Side getLayingSide(const float angleTol);
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
   * @return time [ms] until the motion will be finished
   */
  int standUpMotionFoot();
  /**
   * @brief checkSuccess checks if the stand up motion was successful
   */
  void checkSuccess();
  /**
   * @brief resetStandUp resets the member variables that keep state
   */
  void resetStandUp();
  /**
   * @brief getArmCommandsFromPose extracts the arm joint angles from a pose
   * @param pose a vector containing joint angles for all joints
   * @param rArmCommands the angles for the right arm joints
   * @param lArmCommands the angles for the left arm joints
   */
  void getArmCommandsFromPose(const std::vector<float>& pose, std::vector<float>& rArmCommands,
                              std::vector<float>& lArmCommands);
  /// tolerance of body angle data in degrees when determining ground side
  const Parameter<float> angleTolSideCheck_;
  /// tolerance of body angle data in degrees when determining FmPose
  const Parameter<float> angleTolFmPoseCheck_;
  /// tolerance of body angle data in degrees when determining success
  const Parameter<float> angleTolSuccessCheck_;
  /// [ms] time between two checks of ground side if UNDEFINED
  const Parameter<int> checkingGroundSideInterval_;
  /// maximum number of checking side with result UNDEFINED
  const Parameter<int> maxNumRepeatedSideChecks_;
  /// default standup motion after side check returned UNDEFINED to often
  const Parameter<Side> defaultSideIfCheckFail_;
  /// [ms] time till next success check if not yet successful
  const Parameter<int> checkingSuccessInterval_;
  /// maximum number of waitings for success (otherwise no success)
  const Parameter<int> maxNumRepeatedSuccessChecks_;
  /// speed factor for standUp motion from FOOT
  const Parameter<float> standUpMotionFootSpeed_;
  /// name of motion file containing the needed motion for standing up from back side
  const Parameter<std::string> standUpBackMotionFile_;
  /// name of motion file containing the needed motion for standing up from front side
  const Parameter<std::string> standUpFrontMotionFile_;
  /// a reference to the motion request
  const Dependency<MotionRequest> motionRequest_;
  /// a reference to the motion activation
  const Dependency<MotionActivation> motionActivation_;
  /// a reference to the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// a reference to the IMU sensor data
  const Dependency<IMUSensorData> imuSensorData_;
  /// a reference to the joint sensor data
  const Dependency<JointSensorData> jointSensorData_;
  /// gameController state for transitions
  const Dependency<GameControllerState> gameControllerState_;
  /// a reference to the stand up result
  Production<StandUpResult> standUpResult_;
  /// a reference to the stand up output
  Production<StandUpOutput> standUpOutput_;
  /// state of the StandUp-module
  Status status_;
  /// number of performed side checks
  int numSideChecks_;
  /// number of performed success checks
  int numSuccessChecks_;
  /// [ms] counter of backwards running clock for waiting
  int timerClock_;
  /// angle-data for final position (defined position after the standup motion)
  std::vector<float> finalPose_;
  /// motion-object for whole standup motion if lying on the back side
  MotionFilePlayer standUpMotionBack_;
  /// motion-object for whole standup motion if lying on the front side
  MotionFilePlayer standUpMotionFront_;
  /// an interpolator
  Interpolator interpolator_;
  /// an interpolator
  Interpolator leftArmInterpolatorFirstStage_;
  /// an interpolator
  Interpolator leftArmInterpolatorSecondStage_;
  /// an interpolator
  Interpolator rightArmInterpolatorFirstStage_;
  /// an interpolator
  Interpolator rightArmInterpolatorSecondStage_;
  // needed because Side is a private type
  friend void operator>>(const Uni::Value& in, StandUp::Side& out);
};

/**
 * @brief operator>> is needed for Parameter<Side>
 */
inline void operator>>(const Uni::Value& in, StandUp::Side& out)
{
  out = static_cast<StandUp::Side>(in.asInt32());
}
