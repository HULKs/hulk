#pragma once

#include "Data/BallSearchPosition.hpp"
#include "Data/BallState.hpp"
#include "Data/BishopPosition.hpp"
#include "Data/BodyPose.hpp"
#include "Data/ButtonData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/DefendingPosition.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/HeadMotionOutput.hpp"
#include "Data/KeeperAction.hpp"
#include "Data/MotionState.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/SetPosition.hpp"
#include "Data/StrikerAction.hpp"
#include "Data/SupportingPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"

#include "ActionCommand.hpp"
#include "BehaviorParameters.hpp"


struct DataSet
{
  /**
   * @brief DataSet constructs a DataSet from references to the database
   * @param module a reference to the behavior module
   * @param gcs a reference to the game controller state
   * @param bs a reference to the ball state
   * @param rp a reference to the robot position
   * @param bp a reference to the body pose
   * @param pc a reference to the player configuration
   * @param pr a reference to the playing roles
   * @param ms a reference to the motion state
   * @param hmo a reference to the head motion output
   * @param tbm a reference to the team ball model
   * @param tps a reference to the team players
   * @param fd a reference to the field dimensions
   * @param sa a reference to the striker action
   * @param ka a reference to the keeper action
   * @param ci a reference to the cycle info
   * @param sp a reference to the set position
   * @param dp a reference to the defending position
   * @param bp2 a reference to the bishop position
   * @param sp2 a reference to the supporting position
   * @param bd a reference to the button data
   * @param ws a reference to the world state
   * @param bsp a reference to the ball search position
   * @param lac a reference to the last action command
   */
  DataSet(const ModuleBase& module, const GameControllerState& gcs, const BallState& bs, const RobotPosition& rp, const BodyPose& bp,
          const PlayerConfiguration& pc, const PlayingRoles& pr, const MotionState& ms, const HeadMotionOutput& hmo, const TeamBallModel& tbm,
          const TeamPlayers& tps, const FieldDimensions& fd, const StrikerAction& sa, const KeeperAction& ka, const CycleInfo& ci, const SetPosition& sp,
          const DefendingPosition& dp, const BishopPosition& bp2, const SupportingPosition& sp2, const ButtonData& bd, const WorldState& ws,
          const BallSearchPosition& bsp, const ActionCommand& lac)
    : parameters(module)
    , gameControllerState(gcs)
    , ballState(bs)
    , robotPosition(rp)
    , bodyPose(bp)
    , playerConfiguration(pc)
    , playingRoles(pr)
    , motionState(ms)
    , headMotionOutput(hmo)
    , teamBallModel(tbm)
    , teamPlayers(tps)
    , fieldDimensions(fd)
    , strikerAction(sa)
    , keeperAction(ka)
    , cycleInfo(ci)
    , setPosition(sp)
    , defendingPosition(dp)
    , bishopPosition(bp2)
    , supportingPosition(sp2)
    , buttonData(bd)
    , worldState(ws)
    , ballSearchPosition(bsp)
    , lastActionCommand(lac)
  {
  }
  /// struct to hold parameters of the behavior
  const BehaviorParameters parameters;
  /// a reference to the game controller state
  const GameControllerState& gameControllerState;
  /// a reference to the ball state
  const BallState& ballState;
  /// a reference to the robot position
  const RobotPosition& robotPosition;
  /// a reference to the body pose
  const BodyPose& bodyPose;
  /// a reference to the player configuration
  const PlayerConfiguration& playerConfiguration;
  /// another reference
  const PlayingRoles& playingRoles;
  /// a reference to the motion state
  const MotionState& motionState;
  /// a reference to the head motion output
  const HeadMotionOutput& headMotionOutput;
  /// a reference to the team ball model
  const TeamBallModel& teamBallModel;
  /// a reference to my homies
  const TeamPlayers& teamPlayers;
  /// a reference to the field dimensions
  const FieldDimensions& fieldDimensions;
  /// a reference to the striker action
  const StrikerAction& strikerAction;
  /// a reference to the keeper action
  const KeeperAction& keeperAction;
  /// a reference to the cycle info
  const CycleInfo& cycleInfo;
  /// a reference to the set position
  const SetPosition& setPosition;
  /// a reference to the defending position
  const DefendingPosition& defendingPosition;
  /// a reference to the bishop position
  const BishopPosition& bishopPosition;
  /// a reference to the supporting position
  const SupportingPosition& supportingPosition;
  /// a reference to the button data
  const ButtonData& buttonData;
  /// a reference to the world state
  const WorldState& worldState;
  /// a reference to the ball search position
  const BallSearchPosition& ballSearchPosition;
  /// a reference to the last action command
  const ActionCommand& lastActionCommand;
};
