// ==== FUN_0045b224 @ 0045b224 ====

longlong FUN_0045b224(longlong param_1,undefined8 param_2,ushort param_3,undefined8 param_4,
                     undefined4 *param_5)

{
  int *piVar1;
  int iVar2;
  undefined *puVar3;
  int iVar4;
  longlong lVar5;
  undefined8 uVar6;
  
  *param_5 = 1;
  puVar3 = PTR_DAT_00546000;
  iVar4 = 0;
  lVar5 = FUN_00456560(param_1);
  if (lVar5 != 0) {
    uVar6 = FUN_004bf5b0(lVar5,(uint)param_3,param_1 + 0xc20,1);
    iVar4 = (int)uVar6;
  }
  piVar1 = (int *)(puVar3 + -0xc);
  if (piVar1 != (int *)PTR_DAT_00546008) {
    LOCK();
    iVar2 = *piVar1;
    *piVar1 = *piVar1 + -1;
    UNLOCK();
    if (iVar2 + -1 < 1) {
      thunk_FUN_0050e3a0(piVar1);
    }
  }
  return (longlong)iVar4;
}


