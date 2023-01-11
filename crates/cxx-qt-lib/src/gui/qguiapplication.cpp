// clang-format off
// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// clang-format on
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
// SPDX-FileContributor: Leon Matthes <leon.matthes@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#ifdef CXX_QT_GUI_FEATURE
#include "cxx-qt-lib/qguiapplication.h"

#include <vector>

#include <QtCore/QObject>

// TODO: remove tracing
#include <QtCore/QDebug>

// TODO: asserts

namespace {

class ArgsData : public QObject
{
public:
  char** data() { return m_vector.data(); }
  int size() const { return static_cast<int>(m_vector.size()); }
  void push(std::string string)
  {
    m_ownedVector.push_back(string);
    m_vector.push_back(string.data());
  }

private:
  std::vector<std::string> m_ownedVector;
  std::vector<char*> m_vector;
};

}

namespace rust {
namespace cxxqtlib1 {

::std::int32_t
qguiapplicationExec(QGuiApplication& app)
{
  return static_cast<::std::int32_t>(app.exec());
}

::std::unique_ptr<QGuiApplication>
qguiapplicationNew(::rust::Vec<::rust::String> args)
{
  auto argsData = new ArgsData();
  for (::std::size_t i = 0; i < args.size(); i++) {
    // Construct an owned std::string and copy from the rust::String
    std::string str;
    str.assign(args.at(i).c_str(), args.at(i).size());
    argsData->push(str);
  }
  auto argc = argsData->size();

  auto ptr = ::std::make_unique<QGuiApplication>(argc, argsData->data());
  // Set the parent of the ArgsData to QGuiApplication
  // as the vector needs to live as long as the QGuiApplication
  argsData->setParent(ptr.get());

  // TODO: remove tracing
  // We can access arguments here fine
  qWarning() << Q_FUNC_INFO << qApp->arguments();
  return ptr;
}

}
}
#endif
