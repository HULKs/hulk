/**
 * @file BatteryDisplay.hpp
 * @brief File providing handler for displaying the battery status
 * @author <a href="mailto:finn.poppinga@tuhh.de">Finn Poppinga</a>
 * @author <a href="mailto:oliver.tretau@tuhh.de">Oliver Tretau</a>
 */

#pragma once


/**
 * @brief BatteryDisplay class providing an interface to display battery charge
 */
class BatteryDisplay
{
public:
  /**
   * @brief BatteryDisplay Constructor
   */
  BatteryDisplay();

  /**
   * @brief displayBatteryCharge provides the functionality of displaying
   * @param charge the battery charge level in [0,1]
   * @param current the battery current in Ampere
   * @param leds is filled with the requested head LEDs
   */
  void displayBatteryCharge(const float charge, const float current, float* leds);

private:
  const float smoothness_;
  bool initialized_;            ///< Flag indicating the initilization status
  float smoothedBatteryCharge_; ///< Value representing the smoothed battery charge
  float smoothedBatteryCurrent_;///< Value representing the smoothed charge/discharge current
  unsigned int cycleCount_;     ///< iteration counter
  unsigned int currentBatLed_;  ///<
  float fancyBatteryCharge_;    ///< Value representing the fancy battery charge
  float headLedBrightness_;     ///< Value representing the brightness of each LED
};
