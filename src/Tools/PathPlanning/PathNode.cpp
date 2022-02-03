#include "Tools/PathPlanning/PathNode.hpp"

#include "Framework/Log/Log.hpp"
#include <utility>

PathNode::PathNode(unsigned int id, Vector2f absolutePosition,
                   std::vector<PathObstacle>& pathObstacles, PathObstacle* connectedObstacle,
                   std::weak_ptr<PathNode> pairNode, bool isGoal)
  : id(id)
  , absolutePosition(std::move(absolutePosition))
  , connectedObstacle(connectedObstacle)
  , pairNode(std::move(pairNode))
  , pathObstacles(&pathObstacles)
  , isGoalNode(isGoal)
{
}

bool PathNode::isSameNode(const std::shared_ptr<PathNode>& other) const
{
  // Nodes are the same when their IDs are the same
  return id == other->id;
}

bool PathNode::isGoal(const std::shared_ptr<PathNode>& nodeGoal) const
{
  return isSameNode(nodeGoal);
}

float PathNode::goalDistanceEstimate(const std::shared_ptr<PathNode>& nodeGoal)
{
  return (absolutePosition - nodeGoal->absolutePosition).norm();
}

bool PathNode::getSuccessors(AStarSearch<PathNode>* aStarSearch,
                             const std::shared_ptr<PathNode>& parentNode,
                             const std::shared_ptr<PathNode>& goalNode)
{
  // If this node lies on an obstacle, expand the node
  if (connectedObstacle != nullptr)
  {
    expandOnObstacle(aStarSearch, parentNode, goalNode);
    return true;
  }

  // Otherwise check if the goal node is visible, if so add it as the only successor
  if (isReachable(absolutePosition, goalNode->absolutePosition))
  {
    aStarSearch->addSuccessor(goalNode);
    return true;
  }

  // If this is not a node on an obstacle this has to be the start node.
  // As the goal node isn't visible, expand the start node and find successors on obstacles.
  expandNotOnObstacle(aStarSearch);
  return true;
}

float PathNode::getCost(const std::shared_ptr<PathNode>& successor)
{
  if ((connectedObstacle != nullptr) && (successor->connectedObstacle != nullptr) &&
      *connectedObstacle == *(successor->connectedObstacle))
  {
    // Nodes are on same obstacle, need to walk along the circle
    // we take the stored arc and calculate the cost for it
    bool shortArcAvailable = true;
    Arc<float>& arc = storedArcs.at(successor->id);
    if (!Geometry::isLeftOf(arc.relStart, arc.relEnd))
    {
      shortArcAvailable = false;
    }

    float angle = 0.f;
    // Calculate angle between arc start and endpoint
    [[maybe_unused]] const bool valid =
        Geometry::getAngleBetween(arc.relStart, arc.relEnd, angle, false);
    assert(valid);
    if (!shortArcAvailable)
    {
      angle = 2.f * static_cast<float>(M_PI) - angle;
    }
    assert(angle >= 0.f);
    return angle * connectedObstacle->circle.radius;
  }

  // nodes are not on same obstacle, can just directly go there
  float d = (absolutePosition - successor->absolutePosition).cast<float>().norm();
  assert(d >= 0.f);
  return d;
}

void PathNode::expandNotOnObstacle(AStarSearch<PathNode>* aStarSearch)
{
  for (PathObstacle& obstacle : *pathObstacles)
  {
    // Calculate the tangent points to obstacle
    expandNodeToObstacle(aStarSearch, &obstacle, shared_from_this());
  }
}

void PathNode::expandOnObstacle(AStarSearch<PathNode>* aStarSearch,
                                const std::shared_ptr<PathNode>& parentNode,
                                const std::shared_ptr<PathNode>& goalNode)
{
  if (!connectedObstacle->isPopulated)
  {
    // Populate the obstacle with all nodes on the obstacle
    // via calculating the tangent points to all other obstacles
    std::vector<Line<float>> tangents;
    for (PathObstacle& obstacle : *pathObstacles)
    {
      if (
          // an obstacle is already populated it already holds its nodes so there is no need
          // to populate these
          obstacle.isPopulated
          // Do not check against the same obstacle (checked via memory location)
          || connectedObstacle == &obstacle)
      {
        continue;
      }

      // Calculate the blocked arcs for the connected obstacle
      const std::vector<Arc<float>> arcs =
          Geometry::getBlockedArcs(connectedObstacle->circle, obstacle.circle);
      assert(arcs.size() != 1);
      if (arcs.size() == 2)
      {
        connectedObstacle->blockedArcs.emplace_back(arcs[0]);
        obstacle.blockedArcs.emplace_back(arcs[1]);
      }

      // Calculate the tangents from this obstacle to all others
      tangents.clear();
      Geometry::getTangentsBetweenCircles(connectedObstacle->circle, obstacle.circle, tangents);
      for (const auto& tangent : tangents)
      {
        // Only add the nodes if the tangent does not intersect with other objects
        if (!isReachable(tangent.p1, tangent.p2))
        {
          continue;
        }
        // Create nodes for both obstacles
        const auto nodeOnConnected =
            PathNodeFactory::create(tangent.p1, *pathObstacles, connectedObstacle);
        const auto nodeOnObstacle =
            PathNodeFactory::create(tangent.p2, *pathObstacles, &obstacle, nodeOnConnected);
        nodeOnConnected->pairNode = nodeOnObstacle;

        // Determine chirality of nodes
        Vector2f relativeSelfPosition =
            nodeOnConnected->absolutePosition - connectedObstacle->circle.center;
        Vector2f relativeOtherPosition =
            nodeOnObstacle->absolutePosition - connectedObstacle->circle.center;
        nodeOnConnected->clockwise =
            Geometry::isLeftOf(relativeSelfPosition, relativeOtherPosition);

        relativeSelfPosition = nodeOnObstacle->absolutePosition - obstacle.circle.center;
        relativeOtherPosition = nodeOnConnected->absolutePosition - obstacle.circle.center;
        nodeOnObstacle->clockwise = Geometry::isLeftOf(relativeSelfPosition, relativeOtherPosition);

        connectedObstacle->nodesOnObstacle.emplace_back(nodeOnConnected);
        obstacle.nodesOnObstacle.emplace_back(nodeOnObstacle);
      }
    }
    // Calculate the tangents from the goal node to the obstacle this node is located on
    expandNodeToObstacle(aStarSearch, connectedObstacle, goalNode);

    connectedObstacle->isPopulated = true;
  }
  // Add all other nodes on this obstacle as successors
  for (const auto& node : connectedObstacle->nodesOnObstacle)
  {
    if (
        // Don't add nodes in different direction
        node->clockwise == clockwise
        // Don't add the parent node
        || isSameNode(parentNode)
        // Don't add itself
        || isSameNode(node)
        // Don't add obstructed nodes
        || (storedArcs.count(node->id) == 0 && !findAndStoreFreeArc(node)))
    {
      continue;
    }
    aStarSearch->addSuccessor(node);
  }

  // Add the pair point on other side of tangent as successor as well
  if (auto ptr = pairNode.lock(); !ptr->isSameNode(parentNode))
  {
    // the pair node is always reachable
    aStarSearch->addSuccessor(ptr);
  }
}

void PathNode::expandNodeToObstacle(AStarSearch<PathNode>* aStarSearch, PathObstacle* pathObstacle,
                                    const std::shared_ptr<PathNode>& node)
{
  // If not inside the obstacle, calculate the two tangents to the obstacle
  const std::pair<Vector2f, Vector2f> tangentPoints =
      Geometry::getTangentsOfCircleWithoutChecks(node->absolutePosition, pathObstacle->circle);

  // Select nodeType for the new nodes
  const NodeType pairType = node->isGoalNode                     ? NodeType::GOAL
                            : node->connectedObstacle == nullptr ? NodeType::START
                                                                 : NodeType::OBSTACLE;

  // Create new nodes if they are reachable
  const auto addNodeToAStarSearch = [&, this](const Vector2f& point) {
    if (node->isReachable(node->absolutePosition, point))
    {
      const std::shared_ptr<PathNode> newNode =
          PathNodeFactory::create(point, *pathObstacles, pathObstacle, node);
      newNode->connectedObstacle->nodesOnObstacle.emplace_back(newNode);
      newNode->nodeType = pairType;

      const Vector2f relativeSelfPosition = newNode->absolutePosition - pathObstacle->circle.center;
      const Vector2f relativeOtherPosition = node->absolutePosition - pathObstacle->circle.center;
      newNode->clockwise = Geometry::isLeftOf(relativeSelfPosition, relativeOtherPosition);

      // if the node is created by the goal node we need to check if we can reach it
      if (node->isGoalNode && (clockwise == newNode->clockwise || !findAndStoreFreeArc(newNode)))
      {
        return;
      }
      aStarSearch->addSuccessor(newNode);
    }
  };
  addNodeToAStarSearch(tangentPoints.first);
  addNodeToAStarSearch(tangentPoints.second);
}


bool PathNode::findAndStoreFreeArc(const std::shared_ptr<PathNode>& otherNode)
{
  // Nodes need to be on the same obstacle
  assert(connectedObstacle != nullptr && connectedObstacle == otherNode->connectedObstacle);

  // Always check the short arc first
  Vector2f relStart = absolutePosition - connectedObstacle->circle.center;
  Vector2f relEnd = otherNode->absolutePosition - connectedObstacle->circle.center;
  Vector2f absStart = absolutePosition;
  Vector2f absEnd = otherNode->absolutePosition;
  // Swap endpoints if it was a long arc
  if (!Geometry::isLeftOf(relStart, relEnd))
  {
    std::swap(relStart, relEnd);
    std::swap(absStart, absEnd);
  }

  // We now have the short arc in counter clockwise, check whether it is blocked
  const auto arcBlockingShortArc =
      std::find_if(connectedObstacle->blockedArcs.begin(), connectedObstacle->blockedArcs.end(),
                   [&](const auto& arc) {
                     return (Geometry::isLeftOf(relStart, arc.relStart) &&
                             !Geometry::isLeftOf(relEnd, arc.relStart)) ||
                            (Geometry::isLeftOf(relStart, arc.relEnd) &&
                             !Geometry::isLeftOf(relEnd, arc.relEnd));
                   });
  if (arcBlockingShortArc == connectedObstacle->blockedArcs.end())
  {
    // the arc is not blocked, construct and add the free one
    const Arc<float> freeArc{connectedObstacle->circle, absStart, absEnd, false};
    otherNode->storedArcs[id] = freeArc;
    storedArcs[otherNode->id] = freeArc;
    return true;
  }

  // If we reach this the short arc is blocked
  // Check the long arc
  std::swap(relStart, relEnd);
  std::swap(absStart, absEnd);
  const auto arcBlockingLongArc =
      std::find_if(connectedObstacle->blockedArcs.begin(), connectedObstacle->blockedArcs.end(),
                   [&](const auto& arc) {
                     return !((Geometry::isLeftOf(relEnd, arc.relStart) &&
                               !Geometry::isLeftOf(relStart, arc.relStart)) ||
                              (Geometry::isLeftOf(relEnd, arc.relEnd) &&
                               !Geometry::isLeftOf(relStart, arc.relEnd)));
                   });
  if (arcBlockingLongArc == connectedObstacle->blockedArcs.end())
  {
    const Arc<float> freeArc{connectedObstacle->circle, absStart, absEnd, false};
    otherNode->storedArcs[id] = freeArc;
    storedArcs[otherNode->id] = freeArc;
    return true;
  }

  // All arcs are blocked
  return false;
}

bool PathNode::isReachable(const Vector2f& start, const Vector2f& target) const
{
  // Check for visibility
  const Line<float> line{start, target};
  const auto obstructor =
      std::find_if(pathObstacles->begin(), pathObstacles->end(), [&](const auto& obstacle) {
        return Geometry::hasIntersection(line, obstacle.circle,
                                         std::numeric_limits<float>::epsilon() * 5.f);
      });
  return obstructor == pathObstacles->end();
}

void PathNode::printNodeInfo()
{
  std::cout << "Node position: (" << absolutePosition.x() << ", " << absolutePosition.y() << ")"
            << std::endl;
  if (connectedObstacle != nullptr)
  {
    std::cout << "On obstacle at: " << connectedObstacle->circle.center << std::endl;
  }
  else
  {
    std::cout << "This is the start or end node." << std::endl;
  }
}

void PathNode::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::OBJECT);
  value["id"] << id;
  value["absolutePosition"] << absolutePosition;
  value["clockwise"] << clockwise;
  value["nodeType"] << nodeType;
  value["isGoalNode"] << isGoalNode;
}

std::shared_ptr<PathNode> PathNodeFactory::create(Vector2f absolutePosition,
                                                  std::vector<PathObstacle>& pathObstacles,
                                                  PathObstacle* connectedObstacle,
                                                  std::weak_ptr<PathNode> pairNode, bool isGoal)
{
  return std::make_shared<PathNode>(idCounter__++, std::move(absolutePosition), pathObstacles,
                                    connectedObstacle, pairNode, isGoal);
}

void PathNodeFactory::resetCounter()
{
  idCounter__ = 0;
}

thread_local std::atomic_uint32_t PathNodeFactory::idCounter__ = 0;
