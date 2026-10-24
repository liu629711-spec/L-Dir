// ==== FUN_0045a6a0 @ 0045a6a0 ====

undefined8
FUN_0045a6a0(longlong param_1,ushort param_2,ushort param_3,undefined8 param_4,undefined4 *param_5)

{
  longlong lVar1;
  undefined8 uVar2;
  
  *param_5 = 0;
  lVar1 = FUN_00456560(param_1);
  if (lVar1 == 0) {
    uVar2 = 0;
  }
  else {
    uVar2 = FUN_004c31d4(lVar1,(ulonglong)param_2,param_3,param_4,param_5);
  }
  return uVar2;
}


