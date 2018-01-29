#include <cassert>

#include "SimRobotAdapter.hpp"


SimRobotAdapter* SimRobotAdapter::instance_ = nullptr;

SimRobotAdapter::SimRobotAdapter(SimRobot::Application& simRobot)
  : application_(simRobot)
  , menu_(*this)
{
  assert(instance_ == nullptr);
  instance_ = this;
}

SimRobotAdapter::~SimRobotAdapter()
{
  robots_.clear();
  instance_ = nullptr;
}

bool SimRobotAdapter::compile()
{
  SimRobotCore2::Scene* scene = static_cast<SimRobotCore2::Scene*>(application_.resolveObject("RoboCup", SimRobotCore2::scene));
  if (scene == nullptr)
  {
    return false;
  }
  SimRobot::Object* group = application_.resolveObject("RoboCup.robots", SimRobotCore2::compound);
  unsigned int numberOfObjects = application_.getObjectChildCount(*group);
  if (numberOfObjects < 1)
  {
    return false;
  }
  for (unsigned int i = 0; i < numberOfObjects; i++)
  {
    robots_.emplace_back(std::make_unique<SimRobotInterface>(application_, static_cast<SimRobot::Object*>(application_.getObjectChild(*group, i))));
  }
  return true;
}

void SimRobotAdapter::update()
{
  for (auto& robot : robots_)
  {
    robot->update();
  }
  simulatedTime_ += 10;
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

void SimRobotAdapter::pressHeadButton(const unsigned int index, const HeadButtonType headButtonType)
{
  assert(index < robots_.size());
  robots_[index]->pressHeadButton(headButtonType);
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

unsigned int SimRobotAdapter::getSimulatedTime()
{
  assert(instance_ != nullptr);
  return instance_->simulatedTime_;
}
