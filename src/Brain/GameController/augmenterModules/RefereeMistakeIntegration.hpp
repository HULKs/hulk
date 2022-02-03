#pragma once

#include "Brain/GameController/GCAugmenterInterface.hpp"

#include "Tools/Math/MovingAverage.hpp"

#include "Data/BodyPose.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/TeamBallModel.hpp"

/**
 * @brief RefereeMistakeIntegration searches and corrects common referee mistakes.
 *
 * This is a sub module of GameControllerAugmenter that is responsible for searching and correcting
 * common referee mistakes that can easily be detected by checking the environment
 *
 * setPlay::GOAL_KICK:
 * setPlay::CORNER_KICK:
 * A common mistake is that the referee calls "GOAL KICK RED" while the game controller
 * controller clicks the wrong button. See doc of the integrate*() functions for details on how we
 * attempt to detect / correct this kind of mistakes.
 *
 * unpenalize:
 * A robot is often unpenalized early while the assistant was not ready
 * (e.g. holding the robot in his hand)
 */
class RefereeMistakeIntegration : public GCAugmenterInterface
{
public:
  /**
   * @brief RefereeMistakeIntegration initializes members
   *
   * Uses the given module for registering parameters and dependencies in the module's name.
   *
   * @param module the module to use for parameters and dependencies.
   */
  explicit RefereeMistakeIntegration(ModuleBase& module);

  void cycle(const RawGameControllerState& rawGcState, GameControllerState& gcState) override;

private:
  /**
   * @brief integrateTimeOutAdminMode overrides the gameState whenever there is a TIMEOUT
   *
   * According to the normal game controller implementation it is not possible to have a gameState
   * other than INITIAL when the gamePhase is TIMEOUT. Otherwise the game controller controller is
   * in admin mode. We don't trust the gameState then and override it to be INITIAL (which it should
   * be anyways)
   *
   * @param rawGcState the raw game controller state as received via network
   * @param gcState the current gc state (may already be augmented by other sub modules)
   */
  void integrateTimeOutAdminMode(const RawGameControllerState& rawGcState,
                                 GameControllerState& gcState);

  /**
   * @brief integrateEarlyUnpenalized keeps us penalized when we are high
   *
   * As the game controller controller might press the "unpenalize" button while an assistant still
   * holds the robot in his hands we need to wait until he puts us down on the floor. This is done
   * by extending our penalty until we have ground contact.
   *
   * @param gcState the current gc state (may already be augmented by other sub modules)
   */
  void integrateEarlyUnpenalized(GameControllerState& gcState);

  /**
   * @brief integrateCornerKick checks for logical errors during corner kick situations
   *
   * Will override the rawGameController state if the ball is placed in a corner that does not match
   * the received kicking team information. E.g. when the ball is placed in the enemy corner but
   * they were chosen to be the kicking team.
   *
   * @param rawGcState the raw game controller state as received via network
   * @param gcState the current gc state (may already be augmented by other sub modules)
   */
  void integrateCornerKick(const RawGameControllerState& rawGcState, GameControllerState& gcState);

  /**
   * @brief integrateGoalFreeKick checks for logical errors during goal free kick situations
   *
   * Will override the rawGameController state whenever the ball is in the wrong half of the field
   * during an active goal free kick. E.g. when we do have a goal free kick but the ball is in the
   * enemy half of the field.
   *
   * @param rawGcState the raw game controller state as received via network
   * @param gcState the current gc state (may already be augmented by other sub modules)
   */
  void integrateGoalFreeKick(const RawGameControllerState& rawGcState,
                             GameControllerState& gcState);

  /// TeamBallModel is used to check setPlay logic errors. This needs to be a reference as the team
  /// ball model is needing the game controller state (circular dependencies).
  const Reference<TeamBallModel> teamBallModel_;
  /// The body pose. Used to determine if we were high while we were "unpenalized".
  const Dependency<BodyPose> bodyPose_;
  /// playerConfiguration is used to get the team number (for setting kicking team number)
  const Dependency<PlayerConfiguration> playerConfiguration_;

  /// Whether the ball is in our half (applied hysteresis)
  bool ballInOwnHalf_;
  /// hysteresis for ballInOwnHalf [m]
  const float hysteresis_ = 0.25f;

  /// Average of foot contact for the last 120 frames.
  SimpleArrayMovingAverage<uint8_t, uint16_t, 120> footContactAverage_;

  /// The previous raw game controller state used for detecting state transitions
  RawGameControllerState prevRawGcState_;
  /// The previous game controller state used for detecting state transitions
  /// Note that this is not necessarily the final production of the GCAugmenter
  GameControllerState prevGcState_;
};
