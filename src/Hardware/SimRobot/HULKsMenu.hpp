#pragma once

#include <QObject>

class SimRobotAdapter;
class QMenu;


class HULKsMenu : public QObject
{
  Q_OBJECT
public:
  /**
   * @brief HULKsMenu initializes members
   * @param simRobotAdapter a reference to the SimRobot adapter
   */
  HULKsMenu(SimRobotAdapter& simRobotAdapter);
  /**
   * @brief createUserMenu creates a new menu for HULKs specific purposes
   * @return a Qt menu
   */
  QMenu* createUserMenu() const;

private:
  /// a reference to the SimRobot adapter
  SimRobotAdapter& simRobotAdapter_;
};
