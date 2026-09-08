// Instance administration of the forum, rendered in the core admin console under
// Modules ▸ Forum. It manages OBJECTS — categories, forums, ranks, report
// reasons, censored words, profile fields, FAQ — not scalar settings, so each
// page is a custom section registered through `ModuleAdminRegistry`, exactly as
// mail registers its `addresses`/`dkim` sections. The backend endpoints are the
// same guarded ones (`assert_admin`) the module already exposed.
//
// Each section lives in its own file under `sections/` (one responsibility per
// file); this module only wires them onto their declared admin pages.

import { ModuleAdminRegistry } from '@kubuno/sdk'
import { StructureSection } from './sections/StructureSection'
import { MaintenanceSection } from './sections/MaintenanceSection'
import { RanksSection } from './sections/RanksSection'
import { ReportReasonsSection } from './sections/ReportReasonsSection'
import { CensoredWordsSection } from './sections/CensoredWordsSection'
import { ProfileFieldsSection } from './sections/ProfileFieldsSection'
import { FaqSection } from './sections/FaqSection'

// Registers the forum's admin sections into the core console. Each is mounted on
// its declared `[[setting_groups]]` page WITHOUT a label — the section IS the
// page (it brings its own layout), rather than one tab among several.
export function registerForumAdmin() {
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'structure',
    group:     'structure',
    position:  10,
    Component: StructureSection,
  })
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'maintenance',
    group:     'structure',
    position:  20,
    Component: MaintenanceSection,
  })
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'ranks',
    group:     'ranks',
    position:  10,
    Component: RanksSection,
  })
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'report-reasons',
    group:     'moderation',
    position:  10,
    Component: ReportReasonsSection,
  })
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'censored-words',
    group:     'moderation',
    position:  20,
    Component: CensoredWordsSection,
  })
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'profile-fields',
    group:     'profiles',
    position:  10,
    Component: ProfileFieldsSection,
  })
  ModuleAdminRegistry.register({
    moduleId:  'forum',
    id:        'faq',
    group:     'structure',
    position:  30,
    Component: FaqSection,
  })
}
