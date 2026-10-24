// ==== FUN_0046ab8c @ 0046ab8c ====

longlong FUN_0046ab8c(longlong param_1,undefined8 param_2,undefined8 param_3,ulonglong param_4)

{
  int iVar1;
  int iVar2;
  HRESULT HVar3;
  ulonglong *puVar4;
  int *piVar5;
  longlong *plVar6;
  longlong *plVar7;
  LPCWSTR pWVar8;
  longlong unaff_RBX;
  short sVar9;
  LPCWSTR local_78;
  longlong local_70;
  longlong local_68;
  longlong local_60;
  ulonglong local_58;
  LPITEMIDLIST local_50;
  undefined8 local_48 [2];
  longlong local_38;
  longlong local_30 [2];
  
  sVar9 = (short)param_3;
  FUN_0040331c(&local_68,"");
  FUN_0040331c(&local_70,"");
  iVar2 = FUN_004e37a8();
  if (iVar2 == 0) {
    FUN_004a3f04((ulonglong *)&local_78,0x46,param_3,param_4);
    puVar4 = FUN_004a3f04(&local_58,0xbf,param_3,param_4);
    iVar2 = MessageBoxW(*(HWND *)(param_1 + 8),local_78,(LPCWSTR)*puVar4,0x43);
    piVar5 = (int *)(local_58 - 0xc);
    if (piVar5 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar1 = *piVar5;
      *piVar5 = *piVar5 + -1;
      UNLOCK();
      if (iVar1 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_58 - 0xc));
      }
    }
    if (iVar2 == 2) {
      pWVar8 = local_78 + -6;
      if (pWVar8 != (LPCWSTR)PTR_DAT_00546008) {
        LOCK();
        iVar2 = *(int *)pWVar8;
        *(int *)pWVar8 = *(int *)pWVar8 + -1;
        UNLOCK();
        if (iVar2 + -1 < 1) {
          thunk_FUN_0050e3a0(local_78 + -6);
        }
      }
      piVar5 = (int *)(local_70 + -0xc);
      if (piVar5 != (int *)PTR_DAT_00546008) {
        LOCK();
        iVar2 = *piVar5;
        *piVar5 = *piVar5 + -1;
        UNLOCK();
        if (iVar2 + -1 < 1) {
          thunk_FUN_0050e3a0(piVar5);
        }
      }
      piVar5 = (int *)(local_68 + -0xc);
      if (piVar5 == (int *)PTR_DAT_00546008) {
        return unaff_RBX;
      }
      LOCK();
      iVar2 = *piVar5;
      *piVar5 = *piVar5 + -1;
      UNLOCK();
      if (0 < iVar2 + -1) {
        return unaff_RBX;
      }
      thunk_FUN_0050e3a0(piVar5);
      return unaff_RBX;
    }
    if (iVar2 == 6) {
      FUN_004e37b4();
    }
    pWVar8 = local_78 + -6;
    if (pWVar8 != (LPCWSTR)PTR_DAT_00546008) {
      LOCK();
      iVar2 = *(int *)pWVar8;
      *(int *)pWVar8 = *(int *)pWVar8 + -1;
      UNLOCK();
      if (iVar2 + -1 < 1) {
        thunk_FUN_0050e3a0(local_78 + -6);
      }
    }
  }
  if ((sVar9 == -0x7fcc) && (HVar3 = SHGetSpecialFolderLocation((HWND)0x0,0,&local_50), -1 < HVar3))
  {
    FUN_004efe84(local_48,(ushort *)local_50);
    plVar6 = FUN_004ef70c((longlong)local_48,&local_60);
    FUN_00402154(&local_70,plVar6);
    piVar5 = (int *)(local_60 + -0xc);
    if (piVar5 != (int *)PTR_DAT_00546008) {
      LOCK();
      iVar2 = *piVar5;
      *piVar5 = *piVar5 + -1;
      UNLOCK();
      if (iVar2 + -1 < 1) {
        thunk_FUN_0050e3a0((LPVOID)(local_60 + -0xc));
      }
    }
    FUN_004efeb4(local_48);
  }
  plVar6 = FUN_0040329c(&local_38,&local_68);
  plVar7 = FUN_0040329c(local_30,&local_70);
  FUN_00468b3c(param_1,plVar7,plVar6);
  piVar5 = (int *)(local_70 + -0xc);
  if (piVar5 != (int *)PTR_DAT_00546008) {
    LOCK();
    iVar2 = *piVar5;
    *piVar5 = *piVar5 + -1;
    UNLOCK();
    if (iVar2 + -1 < 1) {
      thunk_FUN_0050e3a0(piVar5);
    }
  }
  piVar5 = (int *)(local_68 + -0xc);
  if (piVar5 != (int *)PTR_DAT_00546008) {
    LOCK();
    iVar2 = *piVar5;
    *piVar5 = *piVar5 + -1;
    UNLOCK();
    if (iVar2 + -1 < 1) {
      thunk_FUN_0050e3a0(piVar5);
    }
  }
  return unaff_RBX;
}


