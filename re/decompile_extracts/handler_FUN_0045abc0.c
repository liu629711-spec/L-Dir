// ==== FUN_0045abc0 @ 0045abc0 ====

undefined8
FUN_0045abc0(undefined8 param_1,undefined8 param_2,short param_3,undefined8 param_4,
            undefined8 *param_5)

{
  int *piVar1;
  LPCWSTR pWVar2;
  undefined8 *puVar3;
  undefined8 *puVar4;
  int iVar5;
  undefined8 uVar6;
  char *pcVar7;
  LPCWSTR local_28 [3];
  
  puVar3 = param_5;
  *(undefined4 *)param_5 = 0;
  param_5 = (undefined8 *)PTR_DAT_00546000;
  if (param_3 == -0x7fbb) {
    pcVar7 = "Connect";
  }
  else {
    if (param_3 != -0x7fba) goto LAB_0045ac18;
    pcVar7 = "Disconnect";
  }
  FUN_004033e8((longlong *)&param_5,pcVar7);
LAB_0045ac18:
  puVar4 = param_5;
  if (*(int *)(param_5 + -1) == 0) {
    piVar1 = (int *)((longlong)param_5 + -0xc);
    if (piVar1 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar5 = *piVar1;
      *piVar1 = *piVar1 + -1;
      UNLOCK();
      if (iVar5 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar1);
      }
    }
    uVar6 = 0;
  }
  else {
    *(undefined4 *)puVar3 = 1;
    local_28[0] = (LPCWSTR)PTR_DAT_00546000;
    piVar1 = (int *)((longlong)param_5 + -0xc);
    iVar5 = lstrlenW(L"shell32.dll,SHHelpShortcuts_RunDLL ");
    FUN_00402204((longlong *)local_28,iVar5,(undefined8 *)L"shell32.dll,SHHelpShortcuts_RunDLL ",
                 *(int *)(puVar4 + -1),puVar4);
    pWVar2 = local_28[0];
    ShellExecuteW((HWND)0x0,L"open",L"rundll32.exe",local_28[0],(LPCWSTR)0x0,5);
    pWVar2 = pWVar2 + -6;
    if (pWVar2 != (LPCWSTR)PTR_DAT_00546008) {
      LOCK();
      iVar5 = *(int *)pWVar2;
      *(int *)pWVar2 = *(int *)pWVar2 + -1;
      UNLOCK();
      if (iVar5 + -1 < 1) {
        thunk_FUN_0050e3a0(pWVar2);
      }
    }
    if (piVar1 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar5 = *piVar1;
      *piVar1 = *piVar1 + -1;
      UNLOCK();
      if (iVar5 + -1 < 1) {
        thunk_FUN_0050e3a0(piVar1);
      }
    }
    uVar6 = 1;
  }
  return uVar6;
}


