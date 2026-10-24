// ==== FUN_00474a34 @ 00474a34 ====

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined8
FUN_00474a34(longlong param_1,undefined8 param_2,ushort param_3,undefined8 param_4,
            undefined4 *param_5)

{
  int iVar1;
  uint uVar2;
  uint uVar3;
  SHORT SVar4;
  undefined8 uVar5;
  longlong *plVar6;
  ulonglong *puVar7;
  longlong *plVar8;
  int *piVar9;
  longlong local_198;
  longlong local_190;
  longlong local_188;
  longlong local_180;
  longlong local_178;
  longlong local_170;
  longlong local_168;
  longlong local_160;
  longlong local_158 [2];
  undefined4 local_148 [2];
  undefined8 local_140;
  undefined8 local_138;
  undefined8 local_128;
  undefined8 local_38;
  
  uVar3 = DAT_00551bf0;
  local_38 = DAT_00547f48;
  uVar2 = DAT_00551bf0 & 1;
  *param_5 = 1;
  if (uVar2 == 0) {
    DAT_00551bf0 = uVar3 | 1;
    DAT_00551be8 = PTR_DAT_00546000;
    FUN_0050ee00(FUN_00523ac8);
  }
  FUN_0050daf0((ulonglong *)local_148,0,0x108);
  local_148[0] = 0x123fa;
  local_140 = 0;
  local_138 = 0;
  local_128 = 0;
  uVar5 = FUN_004ee518(param_1 + 0x7c90,(uint)param_3,(undefined8 *)local_148);
  if ((int)uVar5 != 0) {
    plVar6 = FUN_004eba50(param_1 + 0x7c90,&local_188,(longlong)local_148);
    FUN_00402154((longlong *)&DAT_00551be8,plVar6);
    piVar9 = (int *)(local_188 + -0xc);
    if (piVar9 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *piVar9;
      *piVar9 = *piVar9 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_188 + -0xc));
      }
    }
    if (*(int *)(DAT_00551be8 + -8) != 0) {
      *param_5 = 1;
      SVar4 = GetKeyState(0x11);
      if (SVar4 < 0) {
        FUN_00402154((longlong *)(param_1 + 0xc00),(longlong *)&DAT_00551be8);
        SetTimer(*(HWND *)(param_1 + 8),0x2711,300,(TIMERPROC)0x0);
      }
      else {
        SVar4 = GetKeyState(0x10);
        if (SVar4 < 0) {
          puVar7 = (ulonglong *)FUN_0040329c(local_158,(longlong *)&DAT_00551be8);
          FUN_004a037c(&local_198,puVar7);
          plVar6 = FUN_0040329c(&local_180,(longlong *)&DAT_00551be8);
          FUN_004a0414(&local_190,plVar6);
          plVar6 = FUN_0040329c(&local_168,&local_190);
          plVar8 = FUN_0040329c(&local_178,&local_198);
          FUN_00468b3c(param_1,plVar8,plVar6);
          piVar9 = (int *)(local_190 + -0xc);
          if (piVar9 != (int *)PTR_DAT_00546008) {
            LOCK();
            iVar1 = *piVar9;
            *piVar9 = *piVar9 + -1;
            UNLOCK();
            if (iVar1 + -1 < 1) {
              thunk_FUN_0050e3a0((LPVOID)(local_190 + -0xc));
            }
          }
          piVar9 = (int *)(local_198 + -0xc);
          if (piVar9 != (int *)PTR_DAT_00546008) {
            LOCK();
            iVar1 = *piVar9;
            *piVar9 = *piVar9 + -1;
            UNLOCK();
            if (iVar1 + -1 < 1) {
              thunk_FUN_0050e3a0((LPVOID)(local_198 + -0xc));
            }
          }
        }
        SVar4 = GetAsyncKeyState(0x2e);
        if (SVar4 < 0) {
          plVar6 = FUN_0040329c(&local_170,(longlong *)&DAT_00551be8);
          FUN_00456440(*(HWND *)(param_1 + 8),plVar6);
        }
        else {
          ShowWindow(*(HWND *)(param_1 + 0x368),0);
          plVar6 = FUN_0040329c(&local_160,(longlong *)&DAT_00551be8);
          FUN_0046cc14(param_1,plVar6,1,1);
          ShowWindow(*(HWND *)(param_1 + 0x368),5);
        }
      }
    }
  }
  return 1;
}


