common-abilities-debug-possess = 附身箭矢
    .desc = 射出一个毒箭矢，使你能够控制你的目标。
common-abilities-axe-leap = 斧身一跃
    .desc = 往指定地点跳劈.
common-abilities-hammer-leap = 厄运粉碎
    .desc = 一个拥有击退的AOE（范围）攻击. 跳跃到指定地点.
common-abilities-bow-shotgun = 爆发
    .desc = 迸发一堆箭矢
common-abilities-staff-fireshockwave = 火焰之圈
    .desc = 在地上迸发一圈火焰冲击波
common-abilities-sceptre-wardingaura = 守护光环
    .desc = 守护你盟友，抵御敌人的攻击

# Stances
-heavy_stance = 重剑姿态
-agile_stance = 轻剑姿态
-defensive_stance = 防守姿态
-crippling_stance = 残忍姿态
-cleaving_stance = 劈砍姿态

-basic_state = 基础
-heavy_state = 重型
-agile_state = 轻型
-defensive_state = 防守型
-crippling_state = 残忍型
-cleaving_state = 劈砍型

# Sword abilities
veloren-core-pseudo_abilities-sword-heavy_stance = { -heavy_stance }
    .desc = 使用此姿态可以对震慑敌人，并对震慑的敌人造成更多伤害，但是攻速更慢.
veloren-core-pseudo_abilities-sword-agile_stance = { -agile_stance }
    .desc = 使用此姿态可以使攻击加快，但更微弱.
veloren-core-pseudo_abilities-sword-defensive_stance = { -defensive_stance }
    .desc = 此姿态可微弱的抵御伤害，也可以选择挡开攻击.
veloren-core-pseudo_abilities-sword-crippling_stance = { -crippling_stance }
    .desc = 使用此姿态来攻击可以形成或者加重持久的伤口.
veloren-core-pseudo_abilities-sword-cleaving_stance = { -cleaving_stance }
    .desc = 此姿态可以攻击多个敌人

-double_slash = 二连击

-modified_depending_stance = 会根据姿态而改变形态

veloren-core-pseudo_abilities-sword-double_slash = { -double_slash }
    .desc = 一个二连击.
common-abilities-sword-basic_double_slash = { -basic_state } { -double_slash }
    .desc = 一个基础的二连击.
common-abilities-sword-heavy_double_slash = { -heavy_state } { -double_slash }
    .desc = 一个更慢但可以震慑敌人的二连击.
common-abilities-sword-agile_double_slash = { -agile_state } { -double_slash }
    .desc = 一个极快但更微弱的二连击.
common-abilities-sword-defensive_double_slash = { -defensive_state } { -double_slash }
    .desc = 一个能够减少敌人攻击冲击力的二连击.
common-abilities-sword-crippling_double_slash = { -crippling_state } { -double_slash }
    .desc = 一个能够延长敌人流血的二连击.
common-abilities-sword-cleaving_double_slash = { -cleaving_state } { -double_slash }
    .desc = 一个能够劈穿数个敌人的二连击.
veloren-core-pseudo_abilities-sword-secondary_ability = 副剑技
    .desc = The ability bound to secondary attack key
common-abilities-sword-basic_thrust = 基础突进
    .desc = 蓄势可以使突进更加强大
common-abilities-sword-heavy_slam = 重锤出击
    .desc = A strong overhead slash that can be charged to be more staggering
common-abilities-sword-agile_perforate = Perforate
    .desc = A rapid flurry of light attacks
common-abilities-sword-defensive_vital_jab = { -defensive_state } Vital Jab
    .desc = A quickly charged jab that does more damage against parried foes
common-abilities-sword-crippling_deep_rend = 撕裂伤口
    .desc = A strike aimed at an already open wound, deals more damage to bleeding enemies
common-abilities-sword-cleaving_spiral_slash = 螺旋一劈
    .desc = 挥舞刀刃一圈来攻击附近的任何目标.

-crescent_slash = 空月斩

veloren-core-pseudo_abilities-sword-crescent_slash = { -crescent_slash }
    .desc =
        一个自下而上的斜斩.
        { -modified_depending_stance }
common-abilities-sword-basic_crescent_slash = { -basic_state } { -crescent_slash }
    .desc = 一个基础的自下而上的斜斩.
common-abilities-sword-heavy_crescent_slash = { -heavy_state } { -crescent_slash }
    .desc = 一个自下而上的斜斩, 可能震慑敌人.
common-abilities-sword-agile_crescent_slash = { -agile_state } { -crescent_slash }
    .desc = 一个轻盈且自下而上的斜斩.
common-abilities-sword-defensive_crescent_slash = { -defensive_state } { -crescent_slash }
    .desc = 一个保守型且自下而上的斜斩.
common-abilities-sword-crippling_crescent_slash = { -crippling_state } { -crescent_slash }
    .desc = 一个自下而上的斜斩, 可能使敌人流血.
common-abilities-sword-cleaving_crescent_slash = { -cleaving_state } { -crescent_slash }
    .desc = 一个穿透性强且自下而上的斜斩.

-fell_strike = 坠落打击

veloren-core-pseudo_abilities-sword-fell_strike = { -fell_strike }
    .desc =
        迅速并有力的一斩.
        { -modified_depending_stance }
common-abilities-sword-basic_fell_strike = { -basic_state } { -fell_strike }
    .desc = 一个基础，迅速并有力的一斩.
common-abilities-sword-heavy_fell_strike = { -heavy_state } { -fell_strike }
    .desc = 一个迅速并有力的一斩. 可能震慑敌人.
common-abilities-sword-agile_fell_strike = { -agile_state } { -fell_strike }
    .desc = 一个极其迅速并有力的一斩.
common-abilities-sword-defensive_fell_strike = { -defensive_state } { -fell_strike }
    .desc = A parrying, quick strong slash
common-abilities-sword-crippling_fell_strike = { -crippling_state } { -fell_strike }
    .desc = 一个迅速并有力的一斩. 可能使敌人流血.
common-abilities-sword-cleaving_fell_strike = { -cleaving_state } { -fell_strike }
    .desc = 一个迅速, 有力且能够穿透敌人的一斩.

-skewer = 突刺

veloren-core-pseudo_abilities-sword-skewer = { -skewer }
    .desc =
        一个突刺
        { -modified_depending_stance }
common-abilities-sword-basic_skewer = { -basic_state } { -skewer }
    .desc = 一个基础的突刺.
common-abilities-sword-heavy_skewer = { -heavy_state } { -skewer }
    .desc = 一个可震慑敌人的突刺.
common-abilities-sword-agile_skewer = { -agile_state } { -skewer }
    .desc = 一个迅速的突刺.
common-abilities-sword-defensive_skewer = { -defensive_state } { -skewer }
    .desc = 一个保守型的突刺.
common-abilities-sword-crippling_skewer = { -crippling_state } { -skewer }
    .desc = 一个可导致流血的突刺.
common-abilities-sword-cleaving_skewer = { -cleaving_state } { -skewer }
    .desc = 一个可穿透敌人的突刺.

veloren-core-pseudo_abilities-sword-cascade = Cascade
    .desc =
        An overhead slash
        { -modified_depending_stance }
common-abilities-sword-basic_cascade = { -basic_state } Cascade
    .desc = A basic, overhead slash
common-abilities-sword-heavy_cascade = { -heavy_state } Cascade
    .desc = An overhead slash that can stagger
common-abilities-sword-agile_cascade = { -agile_state } Cascade
    .desc = A quick, overhead slash
common-abilities-sword-defensive_cascade = { -defensive_state } Cascade
    .desc = A parrying, overhead slash
common-abilities-sword-crippling_cascade = { -crippling_state } Cascade
    .desc = An overhead slash that can bleed
common-abilities-sword-cleaving_cascade = { -cleaving_state } Cascade
    .desc = An overhead slash that can cleave through enemies

-cross_cut = X裂斩

veloren-core-pseudo_abilities-sword-cross_cut = { -cross_cut }
    .desc =
        A right and left slash
        { -modified_depending_stance }
common-abilities-sword-basic_cross_cut = { -basic_state } { -cross_cut }
    .desc = A basic right and left slash
common-abilities-sword-heavy_cross_cut = { -heavy_state } { -cross_cut }
    .desc = A right and left slash that can each stagger
common-abilities-sword-agile_cross_cut = { -agile_state } { -cross_cut }
    .desc = A quick right and left slash
common-abilities-sword-defensive_cross_cut = { -defensive_state } { -cross_cut }
    .desc = A parrying right and left slash
common-abilities-sword-crippling_cross_cut = { -crippling_state } { -cross_cut }
    .desc = A right and left slash that can bleed
common-abilities-sword-cleaving_cross_cut = { -cleaving_state } { -cross_cut }
    .desc = A right and left slash which cleave through enemies


-requires_moderate_combo = 需要较多连击方可使用

veloren-core-pseudo_abilities-sword-finisher = 终结
    .desc =
        一个应当在战斗尾声时使用的连击战技
        { -modified_depending_stance }
        终结会根据姿态不同而不同
common-abilities-sword-basic_mighty_strike = 强烈打击
    .desc =
        A simple, powerful slash
        { -requires_moderate_combo }
common-abilities-sword-heavy_guillotine = Guillotine
    .desc =
        A strong cleave that will likely stagger what it doesn't kill
        { -requires_moderate_combo }
common-abilities-sword-agile_hundred_cuts = 百刀切
    .desc =
        对制定目标实行数次极快的刀切
        { -requires_moderate_combo }
common-abilities-sword-defensive_counter = 反击
    .desc =
        A rapidly launched attack that deals substantially more damage to a parried foe
        { -requires_moderate_combo }
common-abilities-sword-crippling_mutilate = 肢解
    .desc =
        Mutilate your foe by sawing through their injuries, deals more damage to bleeding foes
        { -requires_moderate_combo }
common-abilities-sword-cleaving_bladestorm = 飓风之刃
    .desc =
        螺旋式的消灭敌人.
        { -requires_moderate_combo }


-enter_stance = 进入
-require_stance = 需要

common-abilities-sword-heavy_sweep = 重扫
    .desc =
        一个重型并极宽的横斩，对震慑的敌人造成更多伤害.
        { -enter_stance } { -heavy_stance }
common-abilities-sword-heavy_pommel_strike = 钝击
    .desc =
        使用钝器来震荡敌人的头部.
        { -enter_stance } { -heavy_stance }
common-abilities-sword-agile_quick_draw = Quick Draw
    .desc =
        Dash forward as you draw your blade for a quick attack
        { -enter_stance } { -agile_stance }
common-abilities-sword-agile_feint = 佯攻
    .desc =
        先侧移再回击.
        { -enter_stance } { -agile_stance }
common-abilities-sword-defensive_riposte = 还击
    .desc =
        抵挡一次并马上回击.
        { -enter_stance } { -defensive_stance }
common-abilities-sword-defensive_disengage = 撤退
    .desc =
        一击之后向后撤退.
        { -enter_stance } { -defensive_stance }
common-abilities-sword-crippling_gouge = 撕裂
    .desc =
        撕裂敌人并造成一个持续流血的伤口.
        { -enter_stance } { -crippling_stance }
common-abilities-sword-crippling_hamstring = 当筋立断
    .desc =
        对敌人的筋造成伤害，使其机动性降低.
        { -enter_stance } { -crippling_stance }
common-abilities-sword-cleaving_whirlwind_slice = 旋风斩
    .desc =
        螺旋式的攻击你周围的敌人.
        { -enter_stance } { -cleaving_stance }
common-abilities-sword-cleaving_earth_splitter = 地裂斩
    .desc =
        Split the earth, if used while falling will have a much stronger impact
        { -enter_stance } { -cleaving_stance }
common-abilities-sword-heavy_fortitude = 刚毅
    .desc =
        随着受到的伤害增长对于震慑的抵抗力，并提高你震慑的威力.
        { -require_stance } { -heavy_stance }
common-abilities-sword-heavy_pillar_thrust = Pillar Thrust
    .desc =
        Stab your sword down through the enemy, all the way into the ground, is more powerful if used while falling
        { -require_stance } { -heavy_stance }
common-abilities-sword-agile_dancing_edge = 舞动之刃
    .desc =
        使你的攻击与移动更加迅速
        { -require_stance } { -agile_stance }
common-abilities-sword-agile_flurry = 狂潮
    .desc =
        进行数次快速突刺
        { -require_stance } { -agile_stance }
common-abilities-sword-defensive_stalwart_sword = 勇敢之剑
    .desc =
        Shrug off the brunt of attacks, incoming damage is reduced
        { -require_stance } { -defensive_stance }
common-abilities-sword-defensive_deflect = 反弹
    .desc =
        一个极快的格挡，甚至能够能够抵挡弹道
        { -require_stance } { -defensive_stance }
common-abilities-sword-crippling_eviscerate = 二次伤害
    .desc =
        Shreds wounds further, deals more damage to crippled enemies
        { -require_stance } { -crippling_stance }
common-abilities-sword-crippling_bloody_gash = 
    .desc =
        Cruelly strike an already bleeding wound, does more damage to bleeding enemies
        { -require_stance } { -crippling_stance }
common-abilities-sword-cleaving_blade_fever = 狂热刀刃
    .desc =
        Attack more recklessly, increasing the power of your strikes while leaving yourself open to incoming attacks
        { -require_stance } { -cleaving_stance }
common-abilities-sword-cleaving_sky_splitter = 天裂斩
    .desc =
        一个据说能够劈开天空的强力一斩，但会用来劈开敌人.
        { -require_stance } { -cleaving_stance }
