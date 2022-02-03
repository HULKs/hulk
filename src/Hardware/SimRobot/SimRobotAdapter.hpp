#pragma once

#include "Hardware/SimRobot/HULKsMenu.hpp"
#include "Hardware/SimRobot/SimRobotInterface.hpp"
#include <SimRobotCore2.h>
#include <chrono>
#include <memory>
#include <vector>


class SimRobotAdapter : public SimRobot::Module
{
public:
  /**
   * @brief SimRobotAdapter constructs members
   * @param simRobot a reference to the SimRobot application
   */
  explicit SimRobotAdapter(SimRobot::Application& simRobot);
  SimRobotAdapter(const SimRobotAdapter&) = delete;
  SimRobotAdapter(SimRobotAdapter&&) = delete;
  SimRobotAdapter& operator=(const SimRobotAdapter&) = delete;
  SimRobotAdapter& operator=(SimRobotAdapter&&) = delete;
  ~SimRobotAdapter() override = default;
  /**
   * @brief compile is called by SimRobot after loading
   * @return true iff loading was successful
   */
  bool compile() override;
  /**
   * @brief update is called by SimRobot each time step (cycle)
   */
  void update() override;
  /**
   * @brief createUserMenu creates a new menu for HULKs specific purposes
   * @return a Qt menu
   */
  QMenu* createUserMenu() const override;
  /**
   * @brief pressChestButton causes a chest button press on a robot with a given index
   * @param index the index of the robot
   */
  void pressChestButton(unsigned int index);
  /**
   * @brief robotName returns the name of a robot with a given index
   * @param index the index of the robot
   */
  const std::string& robotName(unsigned int index) const;
  /**
   * @brief numberOfRobots returns the number of robots that are simulated
   * @return the number of robots that are simulated
   */
  unsigned int numberOfRobots() const;

private:
  /// a reference to the SimRobot application
  SimRobot::Application& application_;
  /// an object that can generate a menu (e.g. for generating chest button presses)
  HULKsMenu menu_;
  /// the list of simulated robots
  std::vector<std::unique_ptr<SimRobotInterface>> robots_;
  std::chrono::high_resolution_clock::time_point lastUpdate_{};
  std::chrono::high_resolution_clock::duration averageCycleTime_{10ms};
  /// the simulated steps
  std::uint64_t simulatedSteps_{0};
};
