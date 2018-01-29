#include <QMenu>

#include "HULKsMenu.hpp"
#include "SimRobotAdapter.hpp"


HULKsMenu::HULKsMenu(SimRobotAdapter& simRobotAdapter)
  : simRobotAdapter_(simRobotAdapter)
{
}

QMenu* HULKsMenu::createUserMenu() const
{
  QMenu* menu = new QMenu(tr("HULKs"));
  // Chest button
  QAction* allChestButtonsAction = new QAction(tr("Chest Button All Robots"), menu);
  allChestButtonsAction->setShortcut(Qt::Key_C | Qt::CTRL | Qt::SHIFT);
  connect(allChestButtonsAction, &QAction::triggered, this, [this]{
      for (unsigned int i = 0; i < simRobotAdapter_.numberOfRobots(); i++)
      {
        simRobotAdapter_.pressChestButton(i);
      }
    });
  menu->addAction(allChestButtonsAction);
  for (unsigned int i = 0; i < simRobotAdapter_.numberOfRobots(); i++)
  {
    QAction* chestButtonAction = new QAction(tr("Chest Button ") + QString::fromStdString(simRobotAdapter_.robotName(i)), menu);
    connect(chestButtonAction, &QAction::triggered, this, [this, i]{ simRobotAdapter_.pressChestButton(i); });
    menu->addAction(chestButtonAction);
  }
  menu->addSeparator();
  // Head button
  QAction* allHeadButtonsAction = new QAction(tr("Head Button All Robots"), menu);
  allHeadButtonsAction->setShortcut(Qt::Key_H | Qt::CTRL | Qt::SHIFT);
  connect(allHeadButtonsAction, &QAction::triggered, this, [this]{
      for (unsigned int i = 0; i < simRobotAdapter_.numberOfRobots(); i++)
      {
        simRobotAdapter_.pressHeadButton(i, HeadButtonType::FRONT);
      }
    });
  for (unsigned int i = 0; i < simRobotAdapter_.numberOfRobots(); i++)
  {
    QAction* headButtonAction = new QAction(tr("Head Button ") + QString::fromStdString(simRobotAdapter_.robotName(i)), menu);
    connect(headButtonAction, &QAction::triggered, this, [this, i]{ simRobotAdapter_.pressHeadButton(i, HeadButtonType::FRONT); });
    menu->addAction(headButtonAction);
  }
  menu->addAction(allHeadButtonsAction);

  return menu;
}
