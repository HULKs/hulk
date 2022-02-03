#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/WhistleData.hpp"
#include "Framework/Module.hpp"
#include <array>
#include <vector>

class Motion;

class LEDHandler : public Module<LEDHandler, Motion>
{
public:
  ModuleName name__{"LEDHandler"};
  explicit LEDHandler(const ModuleManagerInterface& manager);
  void cycle() override;

private:
  /**
   * @brief createEyeFromMode constructs an Led::Eye from the actioncommand
   * @param led the actioncommand of the led
   * @return the Led::Eye constructed from the request
   */
  static Led::Eye createEyeFromMode(float seconds, const ActionCommand::LED& led);
  /**
   * @brief eyeLEDsColor method providing LED setting for the eye
   * @param color the color to show
   * @return Led::Eye with all colors set to given color
   */
  static Led::Eye eyeLEDsColor(const Led::Color& color);
  /**
   * @brief footLEDs method providing LED setting for the foot
   * @param color the color to show
   * @return Led::Foot with all colors set to given color
   */
  static Led::Foot footLEDs(const Led::Color& color);
  /**
   * @brief eyeRainbow sets the eye LEDs in a fancy rainbow shape
   * @return Led::Eye with rainbow colors
   */
  static Led::Eye eyeRainbow(float seconds);
  /**
   * @brief showRobotStateOnChestLEDs calculates and sets the appropriate chest LED values for a
   * given game state
   * @return Chest button LED with robot state visualized
   */
  Led::Chest showRobotStateOnChestLEDs() const;
  /**
   * @brief getTeamLEDColor calculates the Led::color corresponding to the teamColor
   * @return Led::Color in teamColor
   */
  Led::Color getTeamLEDColor() const;
  /**
   * @brief showKickOffTeamOnRightFootLEDs calculates and sets the appropriate right foot LED values
   * for a given game state
   * @return Led::Color representing the kick off state
   */
  Led::Color showKickOffTeamOnLEDs() const;
  /**
   * @brief showWhistleStatusOnEarLEDs calculates the appropriate ear LED values for a
   * given game state (whistle included)
   * @return pair of ear leds <left, right>
   */
  std::pair<Led::Ear, Led::Ear> showWhistleStatusOnEarLEDs() const;

  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<ActionCommand> actionCommand_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<WhistleData> whistleData_;
};
