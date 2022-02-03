#include <QMenu>

#include "Hardware/SimRobot/HULKsMenu.hpp"
#include "Hardware/SimRobot/SimRobotAdapter.hpp"


HULKsMenu::HULKsMenu(SimRobotAdapter& simRobotAdapter)
  : simRobotAdapter_(simRobotAdapter)
{
}

QMenu* HULKsMenu::createUserMenu() const
{
  auto* menu{new QMenu(tr("HULKs"))};
  // Chest button
  auto* allChestButtonsAction{new QAction(tr("Chest Button All Robots"), menu)};
  allChestButtonsAction->setShortcut(Qt::Key_C | Qt::CTRL | Qt::SHIFT);
  connect(allChestButtonsAction, &QAction::triggered, this, [this] {
    for (unsigned int i = 0; i < simRobotAdapter_.numberOfRobots(); i++)
    {
      simRobotAdapter_.pressChestButton(i);
    }
  });
  menu->addAction(allChestButtonsAction);
  for (unsigned int i = 0; i < simRobotAdapter_.numberOfRobots(); i++)
  {
    auto* chestButtonAction{new QAction(
        tr("Chest Button ") + QString::fromStdString(simRobotAdapter_.robotName(i)), menu)};
    connect(chestButtonAction, &QAction::triggered, this,
            [this, i] { simRobotAdapter_.pressChestButton(i); });
    menu->addAction(chestButtonAction);
  }

  return menu;
}
