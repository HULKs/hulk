#include "Motion/LEDHandler/LEDHandler.hpp"
#include "Tools/Chronometer.hpp"

LEDHandler::LEDHandler(const ModuleManagerInterface& manager)
  : Module(manager)
  , cycleInfo_(*this)
  , actionCommand_(*this)
  , gameControllerState_(*this)
  , whistleData_(*this)
{
}

void LEDHandler::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  static_assert(std::is_same_v<Clock::duration::period, std::chrono::seconds::period>);
  Led::Eye leftEye = createEyeFromMode(cycleInfo_->startTime.time_since_epoch().count(),
                                       actionCommand_->leftLED());
  Led::Eye rightEye = createEyeFromMode(cycleInfo_->startTime.time_since_epoch().count(),
                                        actionCommand_->rightLED());
  const auto chest = showRobotStateOnChestLEDs();
  const auto leftFoot = footLEDs(getTeamLEDColor());
  const auto rightFoot = footLEDs(showKickOffTeamOnLEDs());
  const auto [leftEar, rightEar] = showWhistleStatusOnEarLEDs();
  robotInterface().setLEDs(chest, leftEar, rightEar, leftEye, rightEye, leftFoot, rightFoot);
}

Led::Eye LEDHandler::createEyeFromMode(const float seconds, const ActionCommand::LED& led)
{
  using EyeMode = ActionCommand::LED::EyeMode;
  switch (led.eyeMode)
  {
    case EyeMode::OFF:
      return eyeLEDsColor(Led::Color{0.f, 0.f, 0.f});
    case EyeMode::COLOR:
      return eyeLEDsColor(Led::Color{led.r, led.g, led.b});
    case EyeMode::RAINBOW:
      return eyeRainbow(seconds);
    default:
      assert(false && "Unknown EyeMode requested");
      return eyeLEDsColor(Led::Color{0.f, 0.f, 0.f});
  }
}

Led::Eye LEDHandler::eyeLEDsColor(const Led::Color& color)
{
  Led::Eye eye;
  eye.colorAt0 = color;
  eye.colorAt45 = color;
  eye.colorAt90 = color;
  eye.colorAt135 = color;
  eye.colorAt180 = color;
  eye.colorAt225 = color;
  eye.colorAt270 = color;
  eye.colorAt315 = color;
  return eye;
}

Led::Foot LEDHandler::footLEDs(const Led::Color& color)
{
  Led::Foot foot;
  foot.color = color;
  return foot;
}

/// intervalRatio in [0.0, 1.0)
static Led::Color intervalRatioToRainbowColor(const float intervalRatio)
{
  const auto intervalRatioOver6{intervalRatio * 6.f};
  const auto fraction{intervalRatioOver6 - std::floor(intervalRatioOver6)};
  const auto section{static_cast<std::uint8_t>(intervalRatioOver6)};

  switch (section)
  {
    case 0:
      [[fallthrough]];
    default:
      return Led::Color{
          1.f,
          fraction,
          0.f,
      };
    case 1:
      return Led::Color{
          1.f - fraction,
          1.f,
          0.f,
      };
    case 2:
      return Led::Color{
          0.f,
          1.f,
          fraction,
      };
    case 3:
      return Led::Color{
          0.f,
          1.f - fraction,
          1.f,
      };
    case 4:
      return Led::Color{
          fraction,
          0.f,
          1.f,
      };
    case 5:
      return Led::Color{
          1.f,
          0.f,
          1.f - fraction,
      };
  }
}

Led::Eye LEDHandler::eyeRainbow(const float seconds)
{
  Led::Eye eye;
  const auto fraction{1.f / 8.f};
  const auto offsettedSecondsAt0{seconds - (0.f * fraction)};
  eye.colorAt0 = intervalRatioToRainbowColor(offsettedSecondsAt0 - std::floor(offsettedSecondsAt0));
  const auto offsettedSecondsAt45{seconds - (1.f * fraction)};
  eye.colorAt45 =
      intervalRatioToRainbowColor(offsettedSecondsAt45 - std::floor(offsettedSecondsAt45));
  const auto offsettedSecondsAt90{seconds - (2.f * fraction)};
  eye.colorAt90 =
      intervalRatioToRainbowColor(offsettedSecondsAt90 - std::floor(offsettedSecondsAt90));
  const auto offsettedSecondsAt135{seconds - (3.f * fraction)};
  eye.colorAt135 =
      intervalRatioToRainbowColor(offsettedSecondsAt135 - std::floor(offsettedSecondsAt135));
  const auto offsettedSecondsAt180{seconds - (4.f * fraction)};
  eye.colorAt180 =
      intervalRatioToRainbowColor(offsettedSecondsAt180 - std::floor(offsettedSecondsAt180));
  const auto offsettedSecondsAt225{seconds - (5.f * fraction)};
  eye.colorAt225 =
      intervalRatioToRainbowColor(offsettedSecondsAt225 - std::floor(offsettedSecondsAt225));
  const auto offsettedSecondsAt270{seconds - (6.f * fraction)};
  eye.colorAt270 =
      intervalRatioToRainbowColor(offsettedSecondsAt270 - std::floor(offsettedSecondsAt270));
  const auto offsettedSecondsAt315{seconds - (7.f * fraction)};
  eye.colorAt315 =
      intervalRatioToRainbowColor(offsettedSecondsAt315 - std::floor(offsettedSecondsAt315));
  return eye;
}

Led::Chest LEDHandler::showRobotStateOnChestLEDs() const
{
  Led::Chest chest;

  // See rules section 3.2
  if (gameControllerState_->penalty != Penalty::NONE)
  {
    chest.color = Led::Color{1.f, 0.f, 0.f};
    return chest;
  }
  switch (gameControllerState_->gameState)
  {
    case GameState::INITIAL:
      if (gameControllerState_->chestButtonWasPressedInInitial)
      {
        // Off.
        chest.color = Led::Color{0.f, 0.f, 0.f};
        break;
      }
      // if unstiff, blink blue
      if (std::chrono::duration_cast<std::chrono::seconds>(
              cycleInfo_->getAbsoluteTimeDifference(Clock::time_point{}))
                  .count() %
              2 ==
          0)
      {
        chest.color.blue = 0.0f;
      }
      else
      {
        chest.color.blue = 1.f;
      }
      break;
    case GameState::READY:
      // Blue.
      chest.color.blue = 1.0f;
      break;
    case GameState::SET:
      // Yellow.
      chest.color.red = 1.0f;
      chest.color.green = 0.6f;
      break;
    case GameState::PLAYING:
      // Green.
      chest.color.green = 1.0f;
      break;
    case GameState::FINISHED:
    default:
      // Off.
      break;
  }
  return chest;
}

Led::Color LEDHandler::getTeamLEDColor() const
{
  Led::Color color;

  switch (gameControllerState_->teamColor)
  {
    case TeamColor::BLUE:
      color.blue = 1.0f;
      break;
    case TeamColor::RED:
      color.red = 1.0f;
      break;
    case TeamColor::YELLOW:
      color.red = 1.0f;
      color.green = 0.6f;
      break;
    case TeamColor::BLACK:
      break;
    case TeamColor::WHITE:
      color.red = 1.f;
      color.blue = 1.f;
      color.green = 1.f;
      break;
    case TeamColor::GREEN:
      color.green = 1.0f;
      break;
    case TeamColor::ORANGE:
      color.red = 1.0f;
      color.green = 0.65f;
      break;
    case TeamColor::PURPLE:
      color.red = 0.5f;
      color.blue = 1.0f;
      break;
    case TeamColor::BROWN:
      color.red = 0.15f;
      color.green = 0.15f;
      color.blue = 0.65f;
      break;
    case TeamColor::GRAY:
      color.red = 0.5f;
      color.blue = 0.5f;
      color.green = 0.5f;
      break;
    default:
      break;
  }
  return color;
}

Led::Color LEDHandler::showKickOffTeamOnLEDs() const
{
  const GameState state = gameControllerState_->gameState;
  const bool stateThatRequiresDisplay =
      GameState::INITIAL == state || GameState::READY == state || GameState::SET == state;
  const float value = (gameControllerState_->kickingTeam && stateThatRequiresDisplay) ? 1.0f : 0.0f;

  return {value, value, value};
}

std::pair<Led::Ear, Led::Ear> LEDHandler::showWhistleStatusOnEarLEDs() const
{
  Led::Ear ear;
  // Check for whistle heard in the last second and turn half of the ear LEDs on.
  if (cycleInfo_->getAbsoluteTimeDifference(whistleData_->lastTimeWhistleHeard) < 1s)
  {
    ear.intensityAt0 = 1.f;
    ear.intensityAt0 = 1.f;
    ear.intensityAt36 = 1.f;
    ear.intensityAt72 = 1.f;
    ear.intensityAt108 = 1.f;
    ear.intensityAt144 = 1.f;
    ear.intensityAt180 = 0.f;
    ear.intensityAt216 = 0.f;
    ear.intensityAt252 = 0.f;
    ear.intensityAt288 = 0.f;
    ear.intensityAt324 = 0.f;
    return {ear, ear};
  }
  // Check if we are in the playing state and turn all ear LEDs on.
  if (gameControllerState_->gameState == GameState::PLAYING)
  {
    ear.intensityAt0 = 1.f;
    ear.intensityAt0 = 1.f;
    ear.intensityAt36 = 1.f;
    ear.intensityAt72 = 1.f;
    ear.intensityAt108 = 1.f;
    ear.intensityAt144 = 1.f;
    ear.intensityAt180 = 1.f;
    ear.intensityAt216 = 1.f;
    ear.intensityAt252 = 1.f;
    ear.intensityAt288 = 1.f;
    ear.intensityAt324 = 1.f;
    return {ear, ear};
  }
  ear.intensityAt0 = 1.f;
  ear.intensityAt0 = 1.f;
  ear.intensityAt36 = 0.f;
  ear.intensityAt72 = 0.f;
  ear.intensityAt108 = 0.f;
  ear.intensityAt144 = 0.f;
  ear.intensityAt180 = 0.f;
  ear.intensityAt216 = 0.f;
  ear.intensityAt252 = 0.f;
  ear.intensityAt288 = 0.f;
  ear.intensityAt324 = 0.f;
  return {ear, ear};
}
