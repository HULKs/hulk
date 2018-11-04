#include "Tools/Chronometer.hpp"

#include "LEDHandler.hpp"

using namespace keys::led;

std::array<float, EYE_MAX> LEDHandler::rainbowLeft_ = {
    {0.7f, 0.0f, 0.0f, 0.0f, 0.3f, 1.0f, 1.0f, 1.0f, 0.0f, 0.0f, 0.7f, 1.0f,
     1.0f, 1.0f, 0.3f, 0.0f, 1.0f, 1.0f, 1.0f, 0.5f, 0.0f, 0.0f, 0.0f, 0.5f}};

std::array<float, EYE_MAX> LEDHandler::rainbowRight_ = {
    {0.7f, 1.0f, 1.0f, 1.0f, 0.3f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.3f, 1.0f,
     1.0f, 1.0f, 0.7f, 0.0f, 1.0f, 0.5f, 0.0f, 0.0f, 0.0f, 0.5f, 1.0f, 1.0f}};

LEDHandler::LEDHandler(const ModuleManagerInterface& manager)
  : Module(manager)
  , cycleInfo_(*this)
  , eyeLEDRequest_(*this)
  , gameControllerState_(*this)
  , whistleData_(*this)
  , cmd_(CHEST_MAX + 2 * EAR_MAX + 2 * EYE_MAX + HEAD_MAX + 2 * FOOT_MAX, 0.f)
  , cycleCount_(0)
  , rainbowCycle_(0)
{
}

void LEDHandler::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  if ((cycleCount_ % 20) == 0)
  {
    rainbowCycle_++;
    setEarLeftLEDsCharge(1.f, 1.f);
    setEarRightLEDsCharge(1.f, 1.f);
    switch (eyeLEDRequest_->leftEyeMode)
    {
      case EyeMode::OFF:
        setEyeLeftLEDsColor(0, 0, 0);
        break;
      case EyeMode::COLOR:
        setEyeLeftLEDsColor(eyeLEDRequest_->leftR, eyeLEDRequest_->leftG, eyeLEDRequest_->leftB);
        break;
      case EyeMode::RAINBOW:
        setEyeLeftRainbow();
        break;
    }
    switch (eyeLEDRequest_->rightEyeMode)
    {
      case EyeMode::OFF:
        setEyeRightLEDsColor(0, 0, 0);
        break;
      case EyeMode::COLOR:
        setEyeRightLEDsColor(eyeLEDRequest_->rightR, eyeLEDRequest_->rightG,
                             eyeLEDRequest_->rightB);
        break;
      case EyeMode::RAINBOW:
        setEyeRightRainbow();
        break;
    }
    showRobotStateOnChestLEDs();
    showTeamColorOnLeftFootLEDs();
    showKickOffTeamOnRightFootLEDs();
    showWhistleStatusOnEarLEDs();
    robotInterface().setLEDs(cmd_);
  }
  cycleCount_++;
}

void LEDHandler::setChestLEDs(const float red, const float green, const float blue)
{
  cmd_[0] = blue;
  cmd_[1] = green;
  cmd_[2] = red;
}

void LEDHandler::setEarLeftLEDsCharge(const float charge, const float value)
{
  const unsigned int base = CHEST_MAX;
  const unsigned int ledCount = EAR_MAX * charge;
  for (unsigned int i = 0; i < EAR_MAX; i++)
  {
    if (i < ledCount)
    {
      cmd_[base + i] = value;
    }
    else
    {
      cmd_[base + i] = 0.0f;
    }
  }
}

void LEDHandler::setEarRightLEDsCharge(const float charge, const float value)
{
  const unsigned int base = CHEST_MAX + EAR_MAX;
  const unsigned int ledCount = EAR_MAX * charge;
  for (unsigned int i = 0; i < EAR_MAX; i++)
  {
    if (i < ledCount)
    {
      cmd_[base + i] = value;
    }
    else
    {
      cmd_[base + i] = 0.0f;
    }
  }
}

void LEDHandler::setEyeLeftLEDsColor(const float red, const float green, const float blue)
{
  const unsigned int base = CHEST_MAX + 2 * EAR_MAX;
  for (unsigned int i = 0; i < 8; i++)
  {
    cmd_[base + i] = blue;
    cmd_[base + i + 8] = green;
    cmd_[base + i + 16] = red;
  }
}

void LEDHandler::setEyeRightLEDsColor(const float red, const float green, const float blue)
{
  const unsigned int base = CHEST_MAX + 2 * EAR_MAX + EYE_MAX;
  for (unsigned int i = 0; i < 8; i++)
  {
    cmd_[base + i] = blue;
    cmd_[base + i + 8] = green;
    cmd_[base + i + 16] = red;
  }
}

void LEDHandler::setFootLeftLEDs(const float red, const float green, const float blue)
{
  const unsigned int base = CHEST_MAX + 2 * EAR_MAX + 2 * EYE_MAX + HEAD_MAX;
  cmd_[base] = blue;
  cmd_[base + 1] = green;
  cmd_[base + 2] = red;
}

void LEDHandler::setFootRightLEDs(const float red, const float green, const float blue)
{
  const unsigned int base = CHEST_MAX + 2 * EAR_MAX + 2 * EYE_MAX + HEAD_MAX + FOOT_MAX;
  cmd_[base] = blue;
  cmd_[base + 1] = green;
  cmd_[base + 2] = red;
}

void LEDHandler::setEarLeftLEDs(const float* earSegmentBrightnesses)
{
  // the base index for this led group
  const unsigned int base = CHEST_MAX;
  // update all ear LEDs
  for (uint8_t ledIndex = 0; ledIndex < EAR_MAX; ledIndex++)
  {
    cmd_[base + ledIndex] = earSegmentBrightnesses[ledIndex];
  }
}

void LEDHandler::setEarRightLEDs(const float* earSegmentBrightnesses)
{
  // the base index for this led group
  const unsigned int base = CHEST_MAX + EAR_MAX;
  // update all ear LEDs
  for (uint8_t ledIndex = 0; ledIndex < EAR_MAX; ledIndex++)
  {
    cmd_[base + ledIndex] = earSegmentBrightnesses[ledIndex];
  }
}

void LEDHandler::setEyeLeftRainbow()
{
  for (unsigned int i = 0; i < 8; i++)
  {
    const unsigned int rainbowCycleOffset = 1;
    int l = (rainbowCycle_ + rainbowCycleOffset + i) % 8;
    cmd_.at(CHEST_MAX + 2 * EAR_MAX + i) = rainbowLeft_[l];
    cmd_.at(CHEST_MAX + 2 * EAR_MAX + i + 8) = rainbowLeft_[(l + 8)];
    cmd_.at(CHEST_MAX + 2 * EAR_MAX + i + 16) = rainbowLeft_[(l + 16)];
  }
}

void LEDHandler::setEyeRightRainbow()
{
  for (unsigned int i = 0; i < 8; i++)
  {
    int r = (rainbowCycle_ - i) % 8;
    cmd_.at(CHEST_MAX + 2 * EAR_MAX + EYE_MAX + i) = rainbowLeft_[r];
    cmd_.at(CHEST_MAX + 2 * EAR_MAX + EYE_MAX + i + 8) = rainbowLeft_[r + 8];
    cmd_.at(CHEST_MAX + 2 * EAR_MAX + EYE_MAX + i + 16) = rainbowLeft_[r + 16];
  }
}

void LEDHandler::showRobotStateOnChestLEDs()
{
  float redValue = 0.0f;
  float greenValue = 0.0f;
  float blueValue = 0.0f;

  // See rules section 3.2
  if (gameControllerState_->penalty != Penalty::NONE)
  {
    // Red.
    redValue = 1.0f;
  }
  else
  {
    switch (gameControllerState_->gameState)
    {
      case GameState::INITIAL:
        // Off.
        break;
      case GameState::READY:
        // Blue.
        blueValue = 1.0f;
        break;
      case GameState::SET:
        // Yellow.
        redValue = 1.0f;
        greenValue = 0.6f;
        break;
      case GameState::PLAYING:
        // Green.
        greenValue = 1.0f;
        break;
      case GameState::FINISHED:
      default:
        // Off.
        break;
    }
  }
  setChestLEDs(redValue, greenValue, blueValue);
}

void LEDHandler::showTeamColorOnLeftFootLEDs()
{
  float redValue = 0.0f, blueValue = 0.0f, greenValue = 0.0f;

  switch (gameControllerState_->teamColor)
  {
    case TeamColor::BLUE:
      blueValue = 1.0f;
      break;
    case TeamColor::RED:
      redValue = 1.0f;
      break;
    case TeamColor::YELLOW:
      redValue = 1.0f;
      greenValue = 0.6f;
      break;
    case TeamColor::BLACK:
      break;
    case TeamColor::WHITE:
      redValue = blueValue = greenValue = 1.0f;
      break;
    case TeamColor::GREEN:
      greenValue = 1.0f;
      break;
    case TeamColor::ORANGE:
      redValue = 1.0f;
      greenValue = 0.65f;
      break;
    case TeamColor::PURPLE:
      redValue = 0.5f;
      blueValue = 1.0f;
      break;
    case TeamColor::BROWN:
      redValue = greenValue = 0.15f;
      blueValue = 0.65f;
      break;
    case TeamColor::GRAY:
      redValue = blueValue = greenValue = 0.5f;
      break;
    default:
      break;
  }

  setFootLeftLEDs(redValue, greenValue, blueValue);
}

void LEDHandler::showKickOffTeamOnRightFootLEDs()
{
  const GameState state = gameControllerState_->gameState;
  const bool stateThatRequiresDisplay =
      GameState::INITIAL == state || GameState::READY == state || GameState::SET == state;
  const float value = (gameControllerState_->kickingTeam && stateThatRequiresDisplay) ? 1.0f : 0.0f;

  setFootRightLEDs(value, value, value);
}

void LEDHandler::showWhistleStatusOnEarLEDs()
{
  // Check for whistle heard in the last second and turn half of the ear LEDs on.
  if (cycleInfo_->getTimeDiff(whistleData_->lastTimeWhistleHeard) < 1.f)
  {
    const float halfEars[] = {1.f, 1.f, 1.f, 1.f, 1.f, 0.f, 0.f, 0.f, 0.f, 0.f};
    setEarRightLEDs(halfEars);
    setEarLeftLEDs(halfEars);
  }
  // Check if we are in the playing state and turn all ear LEDs on.
  else if (gameControllerState_->gameState == GameState::PLAYING)
  {
    const float fullEars[] = {1.f, 1.f, 1.f, 1.f, 1.f, 1.f, 1.f, 1.f, 1.f, 1.f};
    setEarRightLEDs(fullEars);
    setEarLeftLEDs(fullEars);
  }
  else
  {
    const float minEars[] = {1.f, 1.f, 0.f, 0.f, 0.f, 0.f, 0.f, 0.f, 0.f, 0.f};
    setEarRightLEDs(minEars);
    setEarLeftLEDs(minEars);
  }
}
