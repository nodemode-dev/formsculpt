# SPDX-License-Identifier: Apache-2.0
# The Unreal Editor runs init_unreal.py from a project's Content/Python folder when it opens (Epic's "Scripting the
# Unreal Editor Using Python"), so copying this folder into a project starts formsculptor's GoF receiver. If the
# project already has an init_unreal.py, add these two lines to it instead.
import formsculptor_gof

formsculptor_gof.start()
