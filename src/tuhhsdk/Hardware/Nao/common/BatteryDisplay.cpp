#include "Definitions/keys.h"

#include "BatteryDisplay.hpp"


BatteryDisplay::BatteryDisplay()
  : smoothness_(0.8f)
  , initialized_(false)
  , cycleCount_(0)
  , currentBatLed_(0)
  , fancyBatteryCharge_(0.0f)
  , headLedBrightness_(0.0f)
{
}

void BatteryDisplay::displayBatteryCharge(const float charge, const float current, float* leds)
{
  if (cycleCount_ == 0)
  {

    if (!initialized_)
    {
      smoothedBatteryCharge_ = charge;
      smoothedBatteryCurrent_ = current;
      initialized_ = true;
    }
    else
    {
      smoothedBatteryCharge_ = smoothness_ * smoothedBatteryCharge_ + (1 - smoothness_) * charge;
      smoothedBatteryCurrent_ = smoothness_ * smoothedBatteryCurrent_ + (1 - smoothness_) * current;
    }

    // Currently charging indicated by slowly filling head LEDs (cycle from 0 to current battery charge)
    if ((smoothedBatteryCurrent_ > 0 && smoothedBatteryCharge_ < 0.95f) || currentBatLed_ != 0)
    {
      headLedBrightness_ = 1.0f;
      currentBatLed_++;

      if (currentBatLed_ / 12.0f > smoothedBatteryCharge_)
      {
        currentBatLed_ = 0;
      }

      fancyBatteryCharge_ = (1.0f / 12.0f) * currentBatLed_;
    }
    // Currently charging but battery is nearly fully charged
    else if (smoothedBatteryCharge_ >= 0.95f && smoothedBatteryCurrent_ >= -0.05f)
    {
      fancyBatteryCharge_ = 1.0f;
      headLedBrightness_ = (headLedBrightness_ <= 0.5f ? 1.0f : 0.1f);
    }
    // Curently not charging (including the possibility that the robot is fully charged)
    else
    {
      headLedBrightness_ = 1.0f;
      fancyBatteryCharge_ = smoothedBatteryCharge_;
    }
  }
  // only update the battery display after 40 cycles.
  cycleCount_++;
  if (cycleCount_ >= 40)
  {
    cycleCount_ = 0;
  }
  const unsigned int ledMax = keys::led::HEAD_MAX * fancyBatteryCharge_;
  for (unsigned int i = 0; i < keys::led::HEAD_MAX; i++)
  {
    if (i < ledMax)
    {
      leds[i] = headLedBrightness_;
    }
    else
    {
      leds[i] = 0.f;
    }
  }
}
