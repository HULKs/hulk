#pragma once

#include "Brain/Behavior/DataSet.hpp"
#include "Data/ActionCommand.hpp"

/**
 * @brief VisionMode selects the mode, active vision should execute
 */
enum class VisionMode
{
  LOOK_AROUND,
  LOOK_AROUND_BALL,
  BALL_TRACKER,
  LOCALIZATION,
  SEARCH_FOR_BALL,
  LOOK_FORWARD,
};
ActionCommand::Head activeVision(const DataSet& d, VisionMode mode);
ActionCommand::Head cameraCalibrationLook(const DataSet& d);

/**
 * @brief Walk behind the ball properly and ensure correct orientation before approaching the ball
 * @param d dataset containing information about the current world state
 * @param target A walk target attached to the ball, usually a kick pose
 * @param velocity The velocity to be used when appraching the ball. Max. velocity is default.
 * @return A walk command to the ball target using the WALK_BEHIND_BALL walking mode
 */
ActionCommand walkBehindBall(const DataSet& d, const Pose& target,
                             const Velocity& velocity = Velocity{});
/**
 * @brief Walk behind ball and dribble it
 * @param d dataset containing information about the current world state
 * @param walkTarget A walk walkTarget attached to the ball, usually a kick pose
 * @param ballTarget relative coordinates specifying the desired destination for the ball
 * @param velocity The velocity to be used when appraching the ball. Max. velocity is default.
 * @return A walk command to the ball walkTarget using the DRIBBLE walking mode
 */
ActionCommand walkBehindBallAndDribble(const DataSet& d, const Pose& walkTarget,
                                       const Vector2f& ballTarget,
                                       const Velocity& velocity = Velocity{});
/**
 * walkToPose calculates the walk request to a given pose
 * It is checked if further movement is needed to be done regarding if robot is close to target
 * Walk commands which results into leaving the field movements are corrected. The target
 * position is modified.
 *
 * @param a reference of type DataSet with actual infos about the robots environment and status
 * @param pose a reference of type Pose containing the coordinates and orientation of the target
 * position
 * @param absolute a flag if pose coordinates are relative to actual robots position or absolute
 * in field coordinates
 * @param walkingMode specifies the mode of operation like following path with fixed orientation
 * @param velocity Desired walking velocities, movement and rotation. [m/s]
 * @param hysteresis the factor by which the target reached thresholds are multiplied if already
 * standing
 * @param fallback the action command that is executed when not walking
 * @return calculated ActionCommand
 */
ActionCommand
walkToPose(const DataSet& d, const Pose& pose, bool absolute = false,
           ActionCommand::Body::WalkMode walkMode = ActionCommand::Body::WalkMode::PATH,
           const Velocity& velocity = Velocity{}, float hysteresis = 2.0,
           const ActionCommand& fallback = ActionCommand::stand());

/**
 * @brief walkToBallAndInWalkKick a skill to perform a basic in-walk kick. As
 * long as the ball is believed to be not kickable this will fall back to
 * walkBehindBall unsing the kickPose as target.
 * @param d the data set containing some references to important data types (e.g. the world model)
 * @param kickPose the kick pose that is to be approached as long as the ball is not kickable
 * @param kickType the type of kick that is to be performed (e.g. forward or turn kick)
 * @param velocity the velocity that is to be used when approaching the ball (full speed if not
 * specified)
 */
ActionCommand walkToBallAndInWalkKick(const DataSet& d, const Pose& kickPose,
                                      BallUtils::Kickable kickable,
                                      InWalkKickType kickType = InWalkKickType::FORWARD,
                                      const Velocity& velocity = Velocity{});

/**
 * @brief walkToBallAndKick creates an action command for walking to the ball and kick it somewhere
 * @pre The team ball has to be seen.
 * @param d a dataset
 * @param kickPose the relative (!!!) kick pose
 * @param kickable the type of kick that is currently executable (may be none)
 * @param ballDestination the position where the ball should end up
 * @param absolute true iff ballDestination is absolute
 * @param velocity the velocity
 * @param kickType the type of kick
 * @return an action command for walking to the ball and kick it somewhere
 */
ActionCommand walkToBallAndKick(const DataSet& d, const Pose& kickPose,
                                BallUtils::Kickable kickable, const Vector2f& ballDestination,
                                bool absolute = false, const Velocity& velocity = Velocity{},
                                KickType kickType = KickType::FORWARD);

ActionCommand kickLeft(const DataSet& d);
ActionCommand kickRight(const DataSet& d);

ActionCommand rotate(const DataSet& d, bool left = true);
ActionCommand rotate(const DataSet& d, float angle, bool isAbsolute);
ActionCommand rotate(const DataSet& d, const Vector2f& target, bool isAbsolute = true);

ActionCommand standUp(const DataSet& d);

ActionCommand bishop(const DataSet& d);
ActionCommand defender(const DataSet& d);
ActionCommand demo(const DataSet& d);
ActionCommand keeper(const DataSet& d);
ActionCommand loser(const DataSet& d);
ActionCommand replacementKeeper(const DataSet& d);
ActionCommand searcher(const DataSet& d);
ActionCommand shootOnHeadTouch(const DataSet& d);
ActionCommand striker(const DataSet& d);
ActionCommand setPlayStriker(const DataSet& d);
ActionCommand supporter(const DataSet& d);

ActionCommand finished(const DataSet& d);
ActionCommand initial(const DataSet& d);
ActionCommand penaltyShootoutStriker(const DataSet& d);
ActionCommand penaltyKeeper(const DataSet& d);
ActionCommand penaltyShootoutPlaying(const DataSet& d);
ActionCommand playSoccer(const DataSet& d);
ActionCommand playing(const DataSet& d);
ActionCommand ready(const DataSet& d);
ActionCommand set(const DataSet& d);

ActionCommand notPenalized(const DataSet& d);

ActionCommand rootBehavior(const DataSet& d);
