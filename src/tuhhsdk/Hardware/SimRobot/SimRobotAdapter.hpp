#pragma once

#include <memory>
#include <vector>

#include <SimRobotCore2.h>

#include "HULKsMenu.hpp"
#include "SimRobotInterface.hpp"


class SimRobotAdapter : public SimRobot::Module
{
public:
  /**
   * @brief SimRobotAdapter constructs members
   * @param simRobot a reference to the SimRobot application
   */
  SimRobotAdapter(SimRobot::Application& simRobot);
  /**
   * @brief ~SimRobotAdapter destroys the object
   */
  ~SimRobotAdapter();
  /**
   * @brief compile is called by SimRobot after loading
   * @return true iff loading was successful
   */
  bool compile();
  /**
   * @brief update is called by SimRobot each time step (cycle)
   */
  void update();
  /**
   * @brief createUserMenu creates a new menu for HULKs specific purposes
   * @return a Qt menu
   */
  QMenu* createUserMenu() const;
  /**
   * @brief pressChestButton causes a chest button press on a robot with a given index
   * @param index the index of the robot
   */
  void pressChestButton(const unsigned int index);
  /**
   * @brief pressHeadButton causes a head button press on a robot with a given index
   * @param index the index of the robot
   * @param headButtonType which head button is pressed
   */
  void pressHeadButton(const unsigned int index, const HeadButtonType headButtonType);
  /**
   * @brief robotName returns the name of a robot with a given index
   * @param index the index of the robot
   */
  const std::string& robotName(const unsigned int index) const;
  /**
   * @brief numberOfRobots returns the number of robots that are simulated
   * @return the number of robots that are simulated
   */
  unsigned int numberOfRobots() const;
  /**
   * @brief getSimulatedTime returns the simulated time
   * @return the simulated milliseconds since the start of the simulation
   */
  static unsigned int getSimulatedTime();
private:
  /// a reference to the SimRobot application
  SimRobot::Application& application_;
  /// an object that can generate a menu (e.g. for generating chest button presses)
  HULKsMenu menu_;
  /// the list of simulated robots
  std::vector<std::unique_ptr<SimRobotInterface>> robots_;
  /// the simulated time (must be != 0 initially because timeBase will be set to it)
  unsigned int simulatedTime_ = 1;
  /// the only instance of this class
  static SimRobotAdapter* instance_;
};
