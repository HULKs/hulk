/**
 * @file LEDHandler.hpp
 * @brief File providing handler for LEDs
 * @author <a href="mailto:oliver.tretau@tuhh.de">Oliver Tretau</a>
 *
 * This file should be used whenever some LED are addressed. Never try to access
 * a LED by using your bare hands (e.g. calling the DcmConnector).
 *
 * Further information on NAOs hardware can be found <a href="http://doc.aldebaran.com/2-1/family/nao_h25/index_h25.html">here</a>.
 */

#pragma once

#include <array>
#include <vector>

#include "Data/CycleInfo.hpp"
#include "Data/EyeLEDRequest.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/WhistleData.hpp"

#include "Definitions/keys.h"
#include "Framework/Module.hpp"


class Motion;

class LEDHandler : public Module<LEDHandler, Motion>
{
public:
  /// the name of this module
  ModuleName name = "LEDHandler";
  LEDHandler(const ModuleManagerInterface& manager);
  void cycle();

private:
  /**
   * @brief setChestLEDs method providing LED setting for the chest
   * @param red a float value specifying the red channel (0.0f-1.0f)
   * @param green a float value specifying the green channel (0.0f-1.0f)
   * @param blue a float value specifying the blue channel (0.0f-1.0f)
   */
  void setChestLEDs(const float red, const float green, const float blue);
  /**
   * @brief setEarLeftLEDsCharge method providing LED setting for the left ear
   * @param charge a float value specifying the percentage of circle fillment (0.0f-1.0f)
   * @param value a float value specifying the intensity (0.0f-1.0f)
   */
  void setEarLeftLEDsCharge(const float charge, const float value);
  /**
   * @brief setEarRightLEDsCharge method providing LED setting for the right ear
   * @param charge a float value specifying the percentage of circle fillment (0.0f-1.0f)
   * @param value a float value specifying the intensity (0.0f-1.0f)
   */
  void setEarRightLEDsCharge(const float charge, const float value);
  /**
   * @brief setEyeLeftLEDs method providing LED setting for the left eye
   * @param red a float value specifying the red channel (0.0f-1.0f)
   * @param green a float value specifying the green channel (0.0f-1.0f)
   * @param blue a float value specifying the blue channel (0.0f-1.0f)
   */
  void setEyeLeftLEDsColor(const float red, const float green, const float blue);
  /**
   * @brief setEyeRightLEDs method providing LED setting for the right eye
   * @param red a float value specifying the red channel (0.0f-1.0f)
   * @param green a float value specifying the green channel (0.0f-1.0f)
   * @param blue a float value specifying the blue channel (0.0f-1.0f)
   */
  void setEyeRightLEDsColor(const float red, const float green, const float blue);
  /**
   * @brief setFootLeftLEDs method providing LED setting for the left foot
   * @param red a float value specifying the red channel (0.0f-1.0f)
   * @param green a float value specifying the green channel (0.0f-1.0f)
   * @param blue a float value specifying the blue channel (0.0f-1.0f)
   */
  void setFootLeftLEDs(const float red, const float green, const float blue);
  /**
   * @brief setFootRightLEDs method providing LED setting for the right foot
   * @param red a float value specifying the red channel (0.0f-1.0f)
   * @param green a float value specifying the green channel (0.0f-1.0f)
   * @param blue a float value specifying the blue channel (0.0f-1.0f)
   */
  void setFootRightLEDs(const float red, const float green, const float blue);
  /**
   * @brief setEarLeftLEDs sets the left ear LEDs to the given brightnesses
   * @param earSegmentBrightnesses the brightness of each ear led
   */
  void setEarLeftLEDs(const float earSegmentBrightnesses[keys::led::EAR_MAX]);
  /**
   * @brief setEarRightLEDs sets the right ear LEDs to the given brightnesses
   * @param earSegmentBrightnesses the brightness of each ear led
   */
  void setEarRightLEDs(const float earSegmentBrightnesses[keys::led::EAR_MAX]);
  /**
   * @brief setEyeLeftRainbow sets the eye LEDs in a fancy rainbow shape
   *
   * Color scheme:
   * Red on L0 and R0 (Left/45Deg and Right/315Deg)
   * Orange/Yellow on L7 and R1 (Left/90Deg and Right/270Deg)
   * Spring Green on L6 and R2 (Left/135Deg and Right/225Deg)
   * Green/Turquoise on L5 and R3 (Left/180Deg and /Right/180Deg)
   * Cyan on L4 and R4 (Left/225Deg and /Right/135Deg)
   * Blue/Ocean on L3 and R5 (Left/270Deg and /Right/90Deg)
   * Violet on L2 and R6 (Left/315Deg and /Right/45Deg)
   * Magenta/Raspberry on L1 and R7 (Left/0Deg and /Right/0Deg)
   *
   * For details on the color and key linkage see:
   * <a href="http://doc.aldebaran.com/2-1/family/nao_h25/index_h25.html">LED Keys NAO H25</a>
   * <a href="http://www.webriti.com/wp-content/uploads/2012/01/rgb-color-wheel-lg.jpg">RGB Color Wheel</a>
   */
  void setEyeLeftRainbow();
  /**
   * @brief setEyeRightRainbow see documentation of setEyeLeftRainbow()
   */
  void setEyeRightRainbow();
  /**
   * @brief showRobotStateOnChestLEDs calculates and sets the appropriate chest LED values for a given game state
   */
  void showRobotStateOnChestLEDs();
  /**
   * @brief showTeamColorOnLeftFootLEDs calculates and sets the appropriate left foot LED values for a given game state
   */
  void showTeamColorOnLeftFootLEDs();
  /**
   * @brief showKickOffTeamOnRightFootLEDs calculates and sets the appropriate right foot LED values for a given game state
   */
  void showKickOffTeamOnRightFootLEDs();
  /**
   * @brief showWhistleStatusOnEarLEDs calculates and sets the appropriate ear LED values for a given game state (whistle included)
   */
  void showWhistleStatusOnEarLEDs();

  /// rainbow colors for left eye
  static std::array<float, keys::led::EYE_MAX> rainbowLeft_;
  /// rainbow colors for right eye
  static std::array<float, keys::led::EYE_MAX> rainbowRight_;

  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<EyeLEDRequest> eyeLEDRequest_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<WhistleData> whistleData_;

  /// the LED command that is assembled (in the order of the LED alias)
  std::vector<float> cmd_;
  /// a cycle counter because LEDs are not sent every cycle
  unsigned int cycleCount_, rainbowCycle_;
};
