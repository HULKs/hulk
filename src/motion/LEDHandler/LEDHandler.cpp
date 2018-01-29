#include "Tools/Chronometer.hpp"

#include "LEDHandler.hpp"

using namespace keys::led;

std::array<float, EYE_MAX> LEDHandler::rainbowLeft_ = {
  { 0.7f, 0.0f, 0.0f, 0.0f, 0.3f, 1.0f, 1.0f, 1.0f,
    0.0f, 0.0f, 0.7f, 1.0f, 1.0f, 1.0f, 0.3f, 0.0f,
    1.0f, 1.0f, 1.0f, 0.5f, 0.0f, 0.0f, 0.0f, 0.5f }
};

std::array<float, EYE_MAX> LEDHandler::rainbowRight_ = {
  { 0.7f, 1.0f, 1.0f, 1.0f, 0.3f, 0.0f, 0.0f, 0.0f,
    0.0f, 0.0f, 0.3f, 1.0f, 1.0f, 1.0f, 0.7f, 0.0f,
    1.0f, 0.5f, 0.0f, 0.0f, 0.0f, 0.5f, 1.0f, 1.0f }
};

LEDHandler::LEDHandler(const ModuleManagerInterface& manager)
  : Module(manager, "LEDHandler")
  , eyeLEDRequest_(*this)
  , gameControllerState_(*this)
  , cmd_(CHEST_MAX + 2 * EAR_MAX + 2 * EYE_MAX + HEAD_MAX + 2 * FOOT_MAX, 0.f)
  , cycleCount_(0)
{
}

void LEDHandler::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  if ((cycleCount_ % 20) == 0)
  {
    setEarLeftLEDsCharge(1.f, 1.f);
    setEarRightLEDsCharge(1.f, 1.f);
    setEyeLeftLEDs(eyeLEDRequest_->leftR, eyeLEDRequest_->leftG, eyeLEDRequest_->leftB);
    setEyeRightLEDs(eyeLEDRequest_->rightR, eyeLEDRequest_->rightG, eyeLEDRequest_->rightB);
    showRobotStateOnChestLEDs();
    showTeamColorOnLeftFootLEDs();
    showKickOffTeamOnRightFootLEDs();
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

void LEDHandler::setEyeLeftLEDs(const float red, const float green, const float blue)
{
  const unsigned int base = CHEST_MAX + 2 * EAR_MAX;
  for (unsigned int i = 0; i < 8; i++)
  {
    cmd_[base + i] = blue;
    cmd_[base + i + 8] = green;
    cmd_[base + i + 16] = red;
  }
}

void LEDHandler::setEyeRightLEDs(const float red, const float green, const float blue)
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

void LEDHandler::setEyeRainbow()
{
  for (unsigned int i = 0; i < 8; i++)
  {
    cmd_.at(CHEST_MAX + 2 * EAR_MAX + i) = rainbowLeft_[i];
    cmd_.at(CHEST_MAX + 2 * EAR_MAX + i + 8) = rainbowLeft_[i + 8];
    cmd_.at(CHEST_MAX + 2 * EAR_MAX + i + 16) = rainbowLeft_[i + 16];
    cmd_.at(CHEST_MAX + 2 * EAR_MAX + EYE_MAX + i) = rainbowRight_[i];
    cmd_.at(CHEST_MAX + 2 * EAR_MAX + EYE_MAX + i + 8) = rainbowRight_[i + 8];
    cmd_.at(CHEST_MAX + 2 * EAR_MAX + EYE_MAX + i + 16) = rainbowRight_[i + 16];
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
    switch (gameControllerState_->state)
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
  const GameState state = gameControllerState_->state;
  const bool stateThatRequiresDisplay = GameState::INITIAL == state || GameState::READY == state || GameState::SET == state;
  const float value = (gameControllerState_->kickoff && stateThatRequiresDisplay) ? 1.0f : 0.0f;

  setFootRightLEDs(value, value, value);
}
