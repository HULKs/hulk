#ifndef _UNIVALUE2JSON_HPP_
#define _UNIVALUE2JSON_HPP_

#include "UniValue.h"
#include "Libs/json/json.h"
#include "print.h"

namespace Uni{
  namespace Converter{
    inline Uni::Value toUniValue(const Json::Value& node ) {
      switch ( node.type() ) {
        case Json::nullValue:
        return Uni::Value( Uni::ValueType::NIL );
        case Json::intValue:
        case Json::uintValue:
        return Uni::Value( node.asInt() );
        case Json::realValue:
        return Uni::Value( node.asDouble() );
        case Json::booleanValue:
        return Uni::Value( node.asBool() );
        case Json::stringValue:
        return Uni::Value( node.asString() );
        case Json::objectValue:
        {
          Uni::Value uniNode(Uni::ValueType::OBJECT);
          for (auto it = node.begin(); it != node.end(); it++) {
            uniNode[it.memberName()] = toUniValue(node[it.memberName()]);
          }
          return uniNode;
        }
        case Json::arrayValue:
        {
          Uni::Value uniNode(Uni::ValueType::ARRAY);
          int i = 0;
          for (auto it = node.begin(); it != node.end(); it++, i++) {
            uniNode[i] = toUniValue(*it);
          }
          return uniNode;
        }
        default:
          throw std::runtime_error("Uni::Converter::toUniValue unhanled type!!");
      }
    }

    inline Json::Value toJson(const Uni::Value& node ) {
      switch ( node.type() ) {
        case Uni::ValueType::NIL:
        return Json::Value( Json::nullValue );
        case Uni::ValueType::INT:
        return Json::Value( node.asInt() );
        case Uni::ValueType::REAL:
        return Json::Value( node.asDouble() );
        case Uni::ValueType::BOOL:
        return Json::Value( node.asBool() );
        case Uni::ValueType::STRING:
        return Json::Value( node.asString() );
        case Uni::ValueType::ARRAY:
        {
          Json::Value jsonNode(Json::arrayValue);
          int i = 0;
          for (auto it = node.listBegin(); it != node.listEnd(); it++, i++)
          {
            jsonNode[i] = toJson(*it);
          }
          return jsonNode;
        }
        case Uni::ValueType::OBJECT:
        {
          Json::Value jsonNode(Json::objectValue);
          for (auto it = node.objectBegin(); it != node.objectEnd(); ++it) {
            jsonNode[it->first] = toJson(it->second);
          }
          return jsonNode;
        }
        default:
          throw std::runtime_error("Uni::Converter::toJson unhanled type!!");
      }
    }
  }
}
#endif //_UNIVALUE2JSON_HPP_
