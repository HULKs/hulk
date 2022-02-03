#include "Hardware/SimRobot/SimRobotAdapter.hpp"
#include "Framework/Log/Log.hpp"
#include <algorithm>
#include <cassert>
#include <thread>

SimRobotAdapter::SimRobotAdapter(SimRobot::Application& simRobot)
  : application_(simRobot)
  , menu_(*this)
{
  lastUpdate_ = std::chrono::high_resolution_clock::now();
}

bool SimRobotAdapter::compile()
{
  auto* const scene{dynamic_cast<SimRobotCore2::Scene*>(
      application_.resolveObject("RoboCup", SimRobotCore2::scene))};
  if (scene == nullptr)
  {
    return false;
  }
  auto* const group{application_.resolveObject("RoboCup.robots", SimRobotCore2::compound)};
  const auto numberOfObjects{application_.getObjectChildCount(*group)};
  if (numberOfObjects < 1)
  {
    return false;
  }
  for (int i{0}; i < numberOfObjects; i++)
  {
    robots_.emplace_back(std::make_unique<SimRobotInterface>(
        application_, static_cast<SimRobot::Object*>(application_.getObjectChild(*group, i))));
  }
  return true;
}

void SimRobotAdapter::update()
{
  for (auto& robot : robots_)
  {
    robot->update(simulatedSteps_);
  }

  const auto now{std::chrono::high_resolution_clock::now()};
  const auto cycleTime{now - lastUpdate_};
  lastUpdate_ = now;

  constexpr auto lowPassFactor{0.01f}; // roughly for 100 Hz
  averageCycleTime_ = averageCycleTime_ +
                      std::chrono::duration_cast<decltype(averageCycleTime_)>(
                          lowPassFactor * std::chrono::duration_cast<std::chrono::duration<float>>(
                                              cycleTime - averageCycleTime_));
  const auto toSleep{10ms - averageCycleTime_};

  if (toSleep > 0s)
  {
    std::this_thread::sleep_for(toSleep);
  }
  ++simulatedSteps_;
}

QMenu* SimRobotAdapter::createUserMenu() const
{
  return menu_.createUserMenu();
}

void SimRobotAdapter::pressChestButton(const unsigned int index)
{
  assert(index < robots_.size());
  robots_[index]->pressChestButton();
}

const std::string& SimRobotAdapter::robotName(const unsigned int index) const
{
  assert(index < robots_.size());
  return robots_[index]->getName();
}

unsigned int SimRobotAdapter::numberOfRobots() const
{
  return robots_.size();
}
