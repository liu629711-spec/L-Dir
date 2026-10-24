// ==== FUN_00456670 @ 00456670 ====

undefined8
FUN_00456670(longlong param_1,undefined8 param_2,uint param_3,undefined8 param_4,undefined4 *param_5
            )

{
  longlong lVar1;
  uint uVar2;
  
  uVar2 = (param_3 & 0xffff) - 0x8041;
  *param_5 = 1;
  if (uVar2 < 4) {
    lVar1 = FUN_00456630(param_1,uVar2);
    if (lVar1 != 0) {
      FUN_004c0d60(lVar1,0,(undefined8 *)0x0,0,1);
    }
  }
  return 1;
}


